use std::sync::{Arc, Condvar, Mutex, RwLock, atomic::{AtomicBool, Ordering}};
use std::thread;

use super::{Memory, ProcessControlBlock, ProcessState};

pub(crate) struct Cpu {
    memory: Arc<RwLock<Memory>>,
    resources: Arc<Mutex<CpuResources>>,
    cycle_should_terminate: Arc<AtomicBool>,
}

impl Cpu {
    pub fn new(memory: Arc<RwLock<Memory>>) -> Cpu {
        let resources = Arc::new(Mutex::new(CpuResources::new()));
        let cycle_should_terminate = Arc::new(AtomicBool::new(false));
        
        let resources_clone = resources.clone();
        let cycle_should_terminate_clone = cycle_should_terminate.clone();

        thread::spawn(move || {
            while !cycle_should_terminate_clone.load(Ordering::Relaxed) {
                let mut resources_clone = resources_clone.lock().unwrap();
                Cpu::cycle(&mut resources_clone);
            }
        });

        Cpu {
            memory,
            resources,
            cycle_should_terminate,
        }
    }

    pub fn execute_process(&mut self, in_pcb: Arc<Mutex<ProcessControlBlock>>, out_pcb: Option<Arc<Mutex<ProcessControlBlock>>>) {
        let mut resources = self.resources.lock().unwrap();
        
        if let Some(out_pcb) = out_pcb {
            let mut out_pcb = out_pcb.lock().unwrap();
            out_pcb.program_counter = resources.program_counter;
            out_pcb.registers.copy_from_slice(&resources.registers);
        }

        let in_pcb = in_pcb.lock().unwrap();

        resources.program_counter = in_pcb.program_counter;
        resources.registers.copy_from_slice(&in_pcb.registers);

        resources.cache = self.memory.read().unwrap().read_block_from(in_pcb.get_mem_start_address(), in_pcb.get_mem_in_start_address());

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

    fn cycle(resources: &mut CpuResources) {
        {
            let (lock, condvar) = &*resources.proc_should_interrupt_condvar;
            let mut should_interrupt = lock.lock().unwrap();

            while *should_interrupt {
                should_interrupt = condvar.wait(should_interrupt).unwrap();
            }
        }

        let current_instruction = resources.cache[resources.program_counter];
        resources.program_counter += 1;

        let decoded_instruction = Cpu::decode(current_instruction);

        Cpu::execute(resources, &decoded_instruction);
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
            0x4 => /* MOV */ Cpu::set_reg(resources, instruction.reg_2_num, Cpu::get_reg(resources, instruction.reg_1_num)),
            0x5 => /* ADD */ Cpu::set_reg(resources, instruction.reg_1_num, Cpu::get_reg(resources, instruction.reg_2_num) + Cpu::get_reg(resources, instruction.reg_3_num)),
            0x6 => /* SUB */ Cpu::set_reg(resources, instruction.reg_1_num, Cpu::get_reg(resources, instruction.reg_2_num) - Cpu::get_reg(resources, instruction.reg_3_num)),
            0x7 => /* MUL */ Cpu::set_reg(resources, instruction.reg_1_num, Cpu::get_reg(resources, instruction.reg_2_num) * Cpu::get_reg(resources, instruction.reg_3_num)),
            0x8 => /* DIV */ Cpu::set_reg(resources, instruction.reg_1_num, Cpu::get_reg(resources, instruction.reg_2_num) / Cpu::get_reg(resources, instruction.reg_3_num)),
            0x9 => /* AND */ Cpu::set_reg(resources, instruction.reg_1_num, Cpu::get_reg(resources, instruction.reg_2_num) & Cpu::get_reg(resources, instruction.reg_3_num)),
            0xA => /* OR */ Cpu::set_reg(resources, instruction.reg_1_num, Cpu::get_reg(resources, instruction.reg_2_num) | Cpu::get_reg(resources, instruction.reg_3_num)),
            0x10 => /* SLT */ {
                if Cpu::get_reg(resources, instruction.reg_1_num) < Cpu::get_reg(resources, instruction.reg_2_num){
                    Cpu::set_reg(resources, instruction.reg_3_num, 1);
                }
                else{
                    Cpu::set_reg(resources, instruction.reg_3_num, 0);
                }
            },
            _ => panic!("Execute error, invalid opcode for arithmetic instruction"),
        };
    }

    fn execute_cond_branch_immediate(resources: &mut CpuResources, instruction: &DecodedInstruction) {
        match instruction.opcode {
            0x2 =>  /* ST */ {
                if Cpu::get_reg(resources, instruction.reg_2_num) == 0 {
                    //self.memory.write_to(instruction.address as usize, Cpu::get_reg(resources, instruction.reg_1_num));
                    Cpu::signal_interrupt(resources, ProcessState::Waiting);
                }
                else {
                    //self.memory.write_to(Cpu::get_reg(resources, instruction.reg_2_num) as usize, Cpu::get_reg(resources, instruction.reg_1_num));
                    Cpu::signal_interrupt(resources, ProcessState::Waiting);
                }
            },
            0x3 =>  /* LW */ {
                if Cpu::get_reg(resources, instruction.reg_2_num) == 0 {
                    //Cpu::set_reg(resources, instruction.reg_1_num, self.fetch(instruction.address));
                    Cpu::signal_interrupt(resources, ProcessState::Waiting);
                }
                else {
                    //Cpu::set_reg(resources, instruction.reg_1_num, self.fetch(Cpu::get_reg(resources, instruction.reg_2_num) as usize));
                    Cpu::signal_interrupt(resources, ProcessState::Waiting);
                }
            },
            0xB =>  /* MOVI */ Cpu::set_reg(resources, instruction.reg_2_num, instruction.address as u32),
            0xC =>  /* ADDI */ Cpu::set_reg(resources, instruction.reg_2_num, Cpu::get_reg(resources, instruction.reg_2_num) + instruction.address as u32),
            0xD =>  /* MULTI */ Cpu::set_reg(resources, instruction.reg_2_num, Cpu::get_reg(resources, instruction.reg_2_num) * instruction.address as u32),
            0xE =>  /* DIVI */ Cpu::set_reg(resources, instruction.reg_2_num, Cpu::get_reg(resources, instruction.reg_2_num) / instruction.address as u32),
            0xF =>  /* LDI */ Cpu::set_reg(resources, instruction.reg_2_num, instruction.address as u32),
            0x11 => /* SLTI */ {
                if Cpu::get_reg(resources, instruction.reg_1_num) < instruction.address as u32 {
                    Cpu::set_reg(resources, instruction.reg_3_num, 1);
                }
                else{
                    Cpu::set_reg(resources, instruction.reg_3_num, 0);
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
        match instruction.opcode {
            0x0 => /* RD */ {
                if Cpu::get_reg(resources, instruction.reg_2_num) == 0 {
                    //Cpu::set_reg(resources, instruction.reg_1_num, self.fetch(instruction.address));
                    Cpu::signal_interrupt(resources, ProcessState::Waiting);
                } else {
                    //Cpu::set_reg(resources, instruction.reg_1_num, self.fetch(Cpu::get_reg(resources, instruction.reg_2_num) as usize));
                    Cpu::signal_interrupt(resources, ProcessState::Waiting);
                }
            },
            0x1 => /* WR */ {
                if Cpu::get_reg(resources, instruction.reg_2_num) == 0 {
                    //self.memory.as_ref().borrow_mut().write_to(instruction.address as usize, Cpu::get_reg(resources, instruction.reg_1_num));
                    Cpu::signal_interrupt(resources, ProcessState::Waiting);
                } else {
                    //self.memory.as_ref().borrow_mut().write_to(Cpu::get_reg(resources, instruction.reg_2_num) as usize, Cpu::get_reg(resources, instruction.reg_1_num));
                    Cpu::signal_interrupt(resources, ProcessState::Waiting);    
                }
            },
            _ => panic!("Execute error, invalid opcode for I/O jump instruction"),
        };
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
    }
}

struct CpuResources {
    cache: Vec<u32>,
    program_counter: usize,
    registers: [u32;16],
    proc_should_interrupt_condvar: Arc<(Mutex<bool>, Condvar)>,
    proc_interrupt_type: ProcessState,
}

impl CpuResources {
    pub fn new() -> CpuResources {
        CpuResources {
            cache: Vec::new(),
            program_counter: 0,
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
