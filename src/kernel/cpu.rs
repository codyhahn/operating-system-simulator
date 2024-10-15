use std::sync::{Arc, Condvar, Mutex, RwLock, atomic::{AtomicBool, Ordering}, mpsc};
use std::thread;

use super::{Memory, ProcessControlBlock, ProcessState};

pub(crate) struct Cpu {
    resources: Arc<Mutex<CpuResources>>,
    cycle_should_terminate: Arc<AtomicBool>,
    dma_channel_handle: Option<thread::JoinHandle<()>>,
}

impl Cpu {
    pub fn new(memory: Arc<RwLock<Memory>>) -> Cpu {
        let memory_clone = memory.clone();

        // DMA thread.
        let (dma_out, dma_in) = mpsc::channel();

        let dma_channel_handle = thread::spawn(move || {
            while let Ok(command) = dma_in.recv() {
                match command {
                    DmaCommand::Read { address, response_sender } => {
                        let memory = memory_clone.read().unwrap();
                        let value = memory.read_from(address);
                        response_sender.send(value).unwrap();
                    },
                    DmaCommand::Write { address, value } => {
                        let mut memory = memory_clone.write().unwrap();
                        memory.write_to(address, value);
                    },
                }
            }
        });

        let resources = Arc::new(Mutex::new(CpuResources::new(memory, dma_out)));
        let cycle_should_terminate = Arc::new(AtomicBool::new(false));
        
        let resources_clone = resources.clone();
        let cycle_should_terminate_clone = cycle_should_terminate.clone();

        // CPU thread.
        thread::spawn(move || {
            while !cycle_should_terminate_clone.load(Ordering::Relaxed) {
                Cpu::cycle(&resources_clone);
            }
        });

        Cpu {
            resources,
            cycle_should_terminate,
            dma_channel_handle: Some(dma_channel_handle),
        }
    }

    pub fn execute_process(&mut self, in_pcb: Arc<Mutex<ProcessControlBlock>>, out_pcb: Option<Arc<Mutex<ProcessControlBlock>>>) {
        let mut resources = self.resources.lock().unwrap();
        
        if let Some(out_pcb) = out_pcb {
            let mut out_pcb = out_pcb.lock().unwrap();
            out_pcb.program_counter = resources.program_counter;
            out_pcb.registers.copy_from_slice(&resources.registers);
            // TODO: Write runtime metrics to out_pcb.
        }

        let in_pcb = in_pcb.lock().unwrap();
        let memory = {
            resources.memory.clone()
        };

        resources.cache = memory.read().unwrap().read_block_from(in_pcb.get_mem_start_address(), in_pcb.get_mem_in_start_address());
        resources.program_counter = in_pcb.program_counter;
        resources.mem_start_address = in_pcb.get_mem_start_address();
        resources.registers.copy_from_slice(&in_pcb.registers);

        let (lock, condvar) = &*resources.proc_should_interrupt_condvar;
        let mut should_interrupt = lock.lock().unwrap();

        *should_interrupt = false;
        condvar.notify_all();
    }

    pub fn await_process_interrupt(&self) -> ProcessState {
        let proc_should_interrupt_condvar = {
            let resources = self.resources.lock().unwrap();
            resources.proc_should_interrupt_condvar.clone()
        };

        let (lock, condvar) = &*proc_should_interrupt_condvar;
        let mut should_interrupt = lock.lock().unwrap();

        while !*should_interrupt {
            should_interrupt = condvar.wait(should_interrupt).unwrap();
        }

        self.resources.lock().unwrap().proc_interrupt_type
    }

    fn cycle(resources: &Arc<Mutex<CpuResources>>) {
        // Sleep until a process is ready to be executed.
        let proc_should_interrupt_convar = {
            let resources = resources.lock().unwrap();
            resources.proc_should_interrupt_condvar.clone()
        };

        {
            let (lock, condvar) = &*proc_should_interrupt_convar;
            let mut should_interrupt = lock.lock().unwrap();

            while *should_interrupt {
                should_interrupt = condvar.wait(should_interrupt).unwrap();
            }
        }

        // Execute instruction.
        let mut resources = resources.lock().unwrap();

        let current_instruction = resources.cache[resources.program_counter]; // Fetch.
        resources.program_counter += 1;

        let decoded_instruction = Cpu::decode(current_instruction);

        Cpu::execute(&mut resources, &decoded_instruction);
    }

    fn decode(instruction: u32) -> DecodedInstruction {
        let mut result = DecodedInstruction::new();
        
        // Get instruction type (bits 0-1).
        result.instr_type = Cpu::extract_bits(instruction, 0, 2).try_into().unwrap();

        // Get opcode (bits 2-6).
        result.opcode = Cpu::extract_bits(instruction, 2, 6).try_into().unwrap();

        // Get register values and address based on instruction type.
        match result.instr_type {
            0b00 => /* Arithmetic */ {
                result.reg_1_num = Cpu::extract_bits(instruction, 8, 4).try_into().unwrap();
                result.reg_2_num = Cpu::extract_bits(instruction, 12, 4).try_into().unwrap();
                result.reg_3_num = Cpu::extract_bits(instruction, 16, 4).try_into().unwrap();
            },
            0b01 => /* Conditional branch or immediate */ {
                result.reg_1_num = Cpu::extract_bits(instruction, 8, 4).try_into().unwrap();
                result.reg_2_num = Cpu::extract_bits(instruction, 12, 4).try_into().unwrap();
                result.address = Cpu::extract_bits(instruction, 16,16).try_into().unwrap();
            },
            0b10 => /* Unconditional jump */ {
                result.address = Cpu::extract_bits(instruction, 8, 16).try_into().unwrap();
            },
            0b11 => /* IO */ {
                result.reg_1_num = Cpu::extract_bits(instruction, 8, 4).try_into().unwrap();
                result.reg_2_num = Cpu::extract_bits(instruction, 12, 4).try_into().unwrap();
                result.address = Cpu::extract_bits(instruction, 16, 16).try_into().unwrap();
            },
            _ => panic!("Decode error, invalid instruction type"),
        }

        // Convert address to word address.
        result.address /= 4;

        result
    }

    fn extract_bits(instruction: u32, start_index: u32, length: u32) -> u32 {
        (instruction << start_index) >> (32 - length)
    }

    fn execute(resources: &mut CpuResources, instruction: &DecodedInstruction) {
        // No-op.
        if instruction.opcode == 0x13 {
            return;
        }

        match instruction.instr_type {
            0b00 => Cpu::execute_arithmetic(resources, instruction),
            0b01 => Cpu::execute_cond_branch_immediate(resources, instruction),
            0b10 => Cpu::execute_uncond_jump(resources, instruction),
            0b11 => Cpu::execute_io(resources, instruction),
            _ => panic!("Execute error, invalid instruction type"),
        }
    }

    fn execute_arithmetic(resources: &mut CpuResources, instruction: &DecodedInstruction) {
        match instruction.opcode {
            0x4 => /* MOV */ Cpu::set_reg(resources, instruction.reg_1_num, Cpu::get_reg(resources, instruction.reg_2_num)),
            0x5 => /* ADD */ Cpu::set_reg(resources, instruction.reg_1_num, Cpu::get_reg(resources, instruction.reg_2_num) + Cpu::get_reg(resources, instruction.reg_3_num)),
            0x6 => /* SUB */ Cpu::set_reg(resources, instruction.reg_1_num, Cpu::get_reg(resources, instruction.reg_2_num) - Cpu::get_reg(resources, instruction.reg_3_num)),
            0x7 => /* MUL */ Cpu::set_reg(resources, instruction.reg_1_num, Cpu::get_reg(resources, instruction.reg_2_num) * Cpu::get_reg(resources, instruction.reg_3_num)),
            0x8 => /* DIV */ Cpu::set_reg(resources, instruction.reg_1_num, Cpu::get_reg(resources, instruction.reg_2_num) / Cpu::get_reg(resources, instruction.reg_3_num)),
            0x9 => /* AND */ Cpu::set_reg(resources, instruction.reg_1_num, Cpu::get_reg(resources, instruction.reg_2_num) & Cpu::get_reg(resources, instruction.reg_3_num)),
            0xA => /* OR */ Cpu::set_reg(resources, instruction.reg_1_num, Cpu::get_reg(resources, instruction.reg_2_num) | Cpu::get_reg(resources, instruction.reg_3_num)),
            0x10 => /* SLT */ {
                if Cpu::get_reg(resources, instruction.reg_2_num) < Cpu::get_reg(resources, instruction.reg_3_num) {
                    Cpu::set_reg(resources, instruction.reg_1_num, 1);
                } else {
                    Cpu::set_reg(resources, instruction.reg_1_num, 0);
                }
            },
            _ => panic!("Execute error, invalid opcode for arithmetic instruction"),
        };
    }

    fn execute_cond_branch_immediate(resources: &mut CpuResources, instruction: &DecodedInstruction) {
        match instruction.opcode {
            0x2 =>  /* ST */ {
                let mut memory = resources.memory.write().unwrap();

                if Cpu::get_reg(resources, instruction.reg_2_num) == 0 {
                    memory.write_to(instruction.address, Cpu::get_reg(resources, instruction.reg_1_num));
                } else {
                    let address = Cpu::get_reg(resources, instruction.reg_2_num) as usize;
                    memory.write_to(address, Cpu::get_reg(resources, instruction.reg_1_num));
                }
            },
            0x3 =>  /* LW */ {
                if Cpu::get_reg(resources, instruction.reg_2_num) == 0 {
                    Cpu::set_reg(resources, instruction.reg_1_num, Cpu::fetch(resources, instruction.address));
                } else {
                    let value = Cpu::fetch(resources, Cpu::get_reg(resources, instruction.reg_2_num) as usize);
                    Cpu::set_reg(resources, instruction.reg_1_num, value);
                }
            },
            0xB =>  /* MOVI */ Cpu::set_reg(resources, instruction.reg_1_num, instruction.address as u32),
            0xC =>  /* ADDI */ Cpu::set_reg(resources, instruction.reg_1_num, Cpu::get_reg(resources, instruction.reg_1_num) + instruction.address as u32),
            0xD =>  /* MULI */ Cpu::set_reg(resources, instruction.reg_1_num, Cpu::get_reg(resources, instruction.reg_1_num) * instruction.address as u32),
            0xE =>  /* DIVI */ Cpu::set_reg(resources, instruction.reg_1_num, Cpu::get_reg(resources, instruction.reg_1_num) / instruction.address as u32),
            0xF =>  /* LDI  */ Cpu::set_reg(resources, instruction.reg_1_num, instruction.address as u32),
            0x11 => /* SLTI */ {
                if Cpu::get_reg(resources, instruction.reg_2_num) < instruction.address as u32 {
                    Cpu::set_reg(resources, instruction.reg_1_num, 1);
                } else {
                    Cpu::set_reg(resources, instruction.reg_1_num, 0);
                }
            },
            0x15 => /* BEQ */ {
                if Cpu::get_reg(resources, instruction.reg_1_num) == Cpu::get_reg(resources, instruction.reg_2_num) {
                    Cpu::branch(resources, instruction.address);
                }
            },
            0x16 => /* BNE */ {
                if Cpu::get_reg(resources, instruction.reg_1_num) != Cpu::get_reg(resources, instruction.reg_2_num) {
                    Cpu::branch(resources, instruction.address);
                }
            },
            0x17 => /* BEZ */ {
                if Cpu::get_reg(resources, instruction.reg_1_num) == 0 {
                    Cpu::branch(resources, instruction.address);
                }
            },
            0x18 => /* BNZ */ {
                if Cpu::get_reg(resources, instruction.reg_1_num) != 0 {
                    Cpu::branch(resources, instruction.address);
                }
            },
            0x19 => /* BGZ */ {
                if Cpu::get_reg(resources, instruction.reg_1_num) > 0 {
                    Cpu::branch(resources, instruction.address);
                }
            },
            0x1A => /* BLZ */ {
                if Cpu::get_reg(resources, instruction.reg_1_num) < 0 {
                    Cpu::branch(resources, instruction.address);
                }
            },
            _ => panic!("Execute error, invalid opcode for conditional branch or immediate instruction"),
        };
    }

    fn execute_uncond_jump(resources: &mut CpuResources, instruction: &DecodedInstruction) {
        match instruction.opcode {
            0x12 => /* HLT */ {
                Cpu::signal_interrupt(resources, ProcessState::Terminated);
            },
            0x14 => /* JMP */ Cpu::branch(resources, instruction.address),
            _ => panic!("Execute error, invalid opcode for unconditional jump instruction"),
        };
    }

    fn execute_io(resources: &mut CpuResources, instruction: &DecodedInstruction) {
        let (response_sender, response_receiver) = mpsc::channel();
        
        match instruction.opcode {
            0x0 => /* RD */ {
                if Cpu::get_reg(resources, instruction.reg_2_num) == 0 {
                    let address = instruction.address;
                    resources.dma_channel.send(DmaCommand::Read { address, response_sender }).unwrap();
                    let value = response_receiver.recv().unwrap();
                    Cpu::set_reg(resources, instruction.reg_1_num, value);
                } else {
                    let address = Cpu::get_reg(resources, instruction.reg_2_num) as usize;
                    resources.dma_channel.send(DmaCommand::Read { address, response_sender }).unwrap();
                    let value = response_receiver.recv().unwrap();
                    Cpu::set_reg(resources, instruction.reg_1_num, value);
                }
            },
            0x1 => /* WR */ {
                if Cpu::get_reg(resources, instruction.reg_2_num) == 0 {
                    let address = instruction.address;
                    let value = Cpu::get_reg(resources, instruction.reg_1_num);
                    resources.dma_channel.send(DmaCommand::Write { address, value }).unwrap();
                } else {
                    let address = Cpu::get_reg(resources, instruction.reg_2_num) as usize;
                    let value = Cpu::get_reg(resources, instruction.reg_1_num);
                    resources.dma_channel.send(DmaCommand::Write { address, value }).unwrap();
                }
            },
            _ => panic!("Execute error, invalid opcode for I/O jump instruction"),
        };
    }

    fn fetch(resources: &CpuResources, address: usize) -> u32 {
        let memory = resources.memory.read().unwrap();
        memory.read_from(address)
    }

    fn branch(resources: &mut CpuResources, destination_address: usize) {
        resources.program_counter = destination_address - 1;
    }

    fn get_reg(resources: &CpuResources, reg_num: usize) -> u32 {
        resources.registers[reg_num]
    }

    fn set_reg(resources: &mut CpuResources, reg_num: usize, value: u32) {
        resources.registers[reg_num] = value;
    }

    fn signal_interrupt(resources: &mut CpuResources, interrupt_type: ProcessState) {
        resources.proc_interrupt_type = interrupt_type;

        let (lock, condvar) = &*resources.proc_should_interrupt_condvar;
        let mut should_interrupt = lock.lock().unwrap();

        *should_interrupt = true;
        condvar.notify_all();
    }
}

impl Drop for Cpu {
    fn drop(&mut self) {
        self.cycle_should_terminate.store(true, Ordering::Relaxed);
        if let Some(dma_channel_handle) = self.dma_channel_handle.take() {
            dma_channel_handle.join().unwrap();
        }
    }
}

enum DmaCommand {
    Read { address: usize, response_sender: mpsc::Sender<u32> },
    Write { address: usize, value: u32 },
}

struct CpuResources {
    memory: Arc<RwLock<Memory>>,
    dma_channel: mpsc::Sender<DmaCommand>,
    cache: Vec<u32>,
    program_counter: usize,
    mem_start_address: usize,
    registers: [u32; 16],
    proc_should_interrupt_condvar: Arc<(Mutex<bool>, Condvar)>,
    proc_interrupt_type: ProcessState,
}

impl CpuResources {
    pub fn new(memory: Arc<RwLock<Memory>>, dma_channel: mpsc::Sender<DmaCommand>) -> CpuResources {
        CpuResources {
            memory,
            dma_channel,
            cache: Vec::new(),
            program_counter: 0,
            mem_start_address: 0,
            registers: [0; 16],
            proc_should_interrupt_condvar: Arc::new((Mutex::new(true), Condvar::new())),
            proc_interrupt_type: ProcessState::Terminated,
        }
    }
}

struct DecodedInstruction {
    instr_type: u8,
    opcode: u8,
    reg_1_num: usize,
    reg_2_num: usize,
    reg_3_num: usize,
    address: usize,
}

impl DecodedInstruction {
    pub fn new() -> DecodedInstruction {
        DecodedInstruction {
            instr_type: 0,
            opcode: 0,
            reg_1_num: 0,
            reg_2_num: 0,
            reg_3_num: 0,
            address: 0,
        }
    }
}

#[cfg(test)]
mod tests {

}
