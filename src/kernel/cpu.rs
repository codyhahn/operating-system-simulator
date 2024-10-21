use std::sync::{Arc, Condvar, Mutex, RwLock, atomic::{AtomicBool, Ordering}, mpsc};
use std::thread;
use std::time::Duration;

use super::{Memory, ProcessControlBlock, ProcessState};

pub(crate) struct Cpu {
    resources: Arc<Mutex<CpuResources>>,
    cycle_should_terminate: Arc<AtomicBool>,
    dma_should_terminate: Arc<AtomicBool>,
    dma_channel_handle: Option<thread::JoinHandle<()>>,
}

impl Cpu {
    pub fn new(memory: Arc<RwLock<Memory>>) -> Cpu {
        let dma_should_terminate = Arc::new(AtomicBool::new(false));

        let memory_clone = memory.clone();
        let dma_should_terminate_clone = dma_should_terminate.clone();

        // DMA thread.
        let (dma_sender, dma_receiver) = mpsc::channel();

        let dma_channel_handle = thread::spawn(move || {
            while !dma_should_terminate_clone.load(Ordering::Relaxed) {
                if let Ok(command) = dma_receiver.recv_timeout(Duration::from_millis(100)) {
                    match command {
                        DmaCommand::Fetch { address, response_sender } => {
                            let memory_clone = memory_clone.read().unwrap();
                            let value = memory_clone.read_from(address);
                            response_sender.send(value).unwrap();
                        },
                        DmaCommand::Store { address, value, response_sender } => {
                            let mut memory_clone = memory_clone.write().unwrap();
                            memory_clone.write_to(address, value);
                            response_sender.send(()).unwrap();
                        },
                    }
                }
            }
        });

        let resources = Arc::new(Mutex::new(CpuResources::new(memory, dma_sender)));
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
            dma_should_terminate,
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

        // Test to see if the program has run for too long. For testing.
        resources.program_timer += 1;
        if resources.program_timer > 1000{
            panic!("Program has executed too many instructions. It's probably in an infinite loop.");
        }
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
            0x4 => /* MOV */ Cpu::set_reg(resources, instruction.reg_1_num, Cpu::get_reg(resources, instruction.reg_2_num)),
            0x5 => /* ADD */ Cpu::set_reg(resources, instruction.reg_3_num, Cpu::get_reg(resources, instruction.reg_1_num) + Cpu::get_reg(resources, instruction.reg_2_num)),
            0x6 => /* SUB */ Cpu::set_reg(resources, instruction.reg_3_num, Cpu::get_reg(resources, instruction.reg_1_num) - Cpu::get_reg(resources, instruction.reg_2_num)),
            0x7 => /* MUL */ Cpu::set_reg(resources, instruction.reg_3_num, Cpu::get_reg(resources, instruction.reg_1_num) * Cpu::get_reg(resources, instruction.reg_2_num)),
            0x8 => /* DIV */ Cpu::set_reg(resources, instruction.reg_3_num, Cpu::get_reg(resources, instruction.reg_1_num) / Cpu::get_reg(resources, instruction.reg_2_num)),
            0x9 => /* AND */ Cpu::set_reg(resources, instruction.reg_3_num, Cpu::get_reg(resources, instruction.reg_1_num) & Cpu::get_reg(resources, instruction.reg_2_num)),
            0xA => /* OR */ Cpu::set_reg(resources, instruction.reg_3_num, Cpu::get_reg(resources, instruction.reg_1_num) | Cpu::get_reg(resources, instruction.reg_2_num)),
            0x10 => /* SLT */ {
                if Cpu::get_reg(resources, instruction.reg_1_num) < Cpu::get_reg(resources, instruction.reg_2_num) {
                    Cpu::set_reg(resources, instruction.reg_3_num, 1);
                } else {
                    Cpu::set_reg(resources, instruction.reg_3_num, 0);
                }
            },
            _ => panic!("Execute error, invalid opcode for arithmetic instruction"),
        };
    }

    fn execute_cond_branch_immediate(resources: &mut CpuResources, instruction: &DecodedInstruction) {
        match instruction.opcode {
            0x2 =>  /* ST */ {
                if instruction.reg_2_num == 0 {
                    let value = Cpu::get_reg(resources, instruction.reg_1_num);
                    Cpu::store(resources, instruction.address, value);
                } else {
                    let address = Cpu::get_reg(resources, instruction.reg_2_num) as usize;
                    let value = Cpu::get_reg(resources, instruction.reg_1_num);
                    Cpu::store(resources, address, value);
                }
            },
            0x3 =>  /* LW */ {
                if instruction.reg_1_num == 0 {
                    let value = Cpu::fetch(resources, instruction.address);
                    Cpu::set_reg(resources, instruction.reg_2_num, value);
                } else {
                    let address = Cpu::get_reg(resources, instruction.reg_1_num) as usize;
                    let value = Cpu::fetch(resources, address);
                    Cpu::set_reg(resources, instruction.reg_2_num, value);
                }
            },
            0xB =>  /* MOVI */ Cpu::set_reg(resources, instruction.reg_2_num, instruction.address as u32),
            0xC =>  /* ADDI */ Cpu::set_reg(resources, instruction.reg_2_num, Cpu::get_reg(resources, instruction.reg_2_num) + instruction.address as u32),
            0xD =>  /* MULI */ Cpu::set_reg(resources, instruction.reg_2_num, Cpu::get_reg(resources, instruction.reg_2_num) * instruction.address as u32),
            0xE =>  /* DIVI */ Cpu::set_reg(resources, instruction.reg_2_num, Cpu::get_reg(resources, instruction.reg_2_num) / instruction.address as u32),
            0xF =>  /* LDI  */ Cpu::set_reg(resources, instruction.reg_2_num, instruction.address as u32),
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
        match instruction.opcode {
            0x0 => /* RD */ {
                let (response_sender, response_receiver) = mpsc::channel();

                // Register 0 is the accumulator, which will never be used as a pointer.
                if instruction.reg_2_num == 0 {
                    let address = instruction.address / 4 + resources.mem_start_address;
                    resources.dma_sender.send(DmaCommand::Fetch { address, response_sender }).unwrap();
                    let value = response_receiver.recv().unwrap();
                    Cpu::set_reg(resources, instruction.reg_1_num, value);
                } else {
                // If that register (reg2) is not zero, assume it's a pointer and use its contents as a memory address.
                    let address = Cpu::get_reg(resources, instruction.reg_2_num) as usize / 4 + resources.mem_start_address;
                    resources.dma_sender.send(DmaCommand::Fetch { address, response_sender }).unwrap();
                    let value = response_receiver.recv().unwrap();
                    Cpu::set_reg(resources, instruction.reg_1_num, value);
                }
            },
            0x1 => /* WR */ {
                let (response_sender, response_receiver) = mpsc::channel();

                // Register 0 is the accumulator, which will never be used as a pointer.
                if instruction.reg_2_num == 0 {
                    let address = instruction.address / 4 + resources.mem_start_address;
                    let value = Cpu::get_reg(resources, instruction.reg_1_num);
                    resources.dma_sender.send(DmaCommand::Store { address, value, response_sender }).unwrap();
                    response_receiver.recv().unwrap();
                } else {
                // If that register (reg2) is not zero, assume it's a pointer and use its contents as a memory address.
                    let address = Cpu::get_reg(resources, instruction.reg_2_num) as usize / 4 + resources.mem_start_address;
                    let value = Cpu::get_reg(resources, instruction.reg_1_num);
                    resources.dma_sender.send(DmaCommand::Store { address, value, response_sender }).unwrap();
                    response_receiver.recv().unwrap();
                }
            },
            _ => panic!("Execute error, invalid opcode for I/O jump instruction"),
        };
    }

    fn fetch(resources: &CpuResources, address: usize) -> u32 {
        let memory = resources.memory.read().unwrap();
        let address = Cpu::get_physical_address_for(resources, address / 4);
        memory.read_from(address)
    }

    fn store(resources: &mut CpuResources, address: usize, value: u32) {
        let mut memory = resources.memory.write().unwrap();
        let address = Cpu::get_physical_address_for(resources, address / 4);
        memory.write_to(address, value);
    }

    fn get_physical_address_for(resources: &CpuResources, logical_address: usize) -> usize {
        logical_address + resources.mem_start_address
    }

    fn branch(resources: &mut CpuResources, destination_address: usize) {
        resources.program_counter = destination_address / 4 /*- 1*/;
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
        self.dma_should_terminate.store(true, Ordering::Relaxed);
        if let Some(dma_channel_handle) = self.dma_channel_handle.take() {
            dma_channel_handle.join().unwrap();
        }
    }
}

enum DmaCommand {
    Fetch { address: usize, response_sender: mpsc::Sender<u32> },
    Store { address: usize, value: u32, response_sender: mpsc::Sender<()> },
}

struct CpuResources {
    memory: Arc<RwLock<Memory>>,
    dma_sender: mpsc::Sender<DmaCommand>,
    cache: Vec<u32>,
    program_counter: usize,
    mem_start_address: usize,
    registers: [u32; 16],
    proc_should_interrupt_condvar: Arc<(Mutex<bool>, Condvar)>,
    proc_interrupt_type: ProcessState,
    program_timer:u32,
}

impl CpuResources {
    pub fn new(memory: Arc<RwLock<Memory>>, dma_sender: mpsc::Sender<DmaCommand>) -> CpuResources {
        CpuResources {
            memory,
            dma_sender,
            cache: Vec::new(),
            program_counter: 0,
            mem_start_address: 0,
            registers: [0; 16],
            proc_should_interrupt_condvar: Arc::new((Mutex::new(true), Condvar::new())),
            proc_interrupt_type: ProcessState::Terminated,
            program_timer:0,
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
    use super::*;

    use crate::io::ProgramInfo;

    #[test]
    fn test_execute_job1() {

        println!("\nBegin CPU Test\n");

        let job1_program_info = ProgramInfo {
            id: 0,
            priority: 0,
            instruction_buffer_size: 23,
            in_buffer_size: 20,
            out_buffer_size: 12,
            temp_buffer_size: 12,
            data_start_idx: 23,
        };

        // This is // JOB 1 from data/program_file.txt. It's supposed to copy the input array and then sum the numbers.
        let job1_program_data: [u32; 100] = [
            0xC050005C,
            0x4B060000,
            0x4B010000,
            0x4B000000,
            0x4F0A005C,
            0x4F0D00DC,
            0x4C0A0004,
            0xC0BA0000,
            0x42BD0000,
            0x4C0D0004,
            0x4C060001,
            0x10658000,
            0x56810018,
            0x4B060000,
            0x4F0900DC,
            0x43970000,
            0x05070000,
            0x4C060001,
            0x4C090004,
            0x10658000,
            0x5681003C,
            0xC10000AC,
            0x92000000,
            0x0000000A,
            0x00000006,
            0x0000002C,
            0x00000045,
            0x00000001,
            0x00000007,
            0x00000000,
            0x00000001,
            0x00000005,
            0x0000000A,
            0x00000055,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
        ];

        println!("\nTesting job number 1: summation");


        let mut memory = Memory::new();
    
        memory.create_process(&job1_program_info, &job1_program_data);
        let pcb = memory.get_pcb_for(0);
    
        let memory = Arc::new(RwLock::new(memory));
        let mut cpu = Cpu::new(memory.clone());
    
        cpu.execute_process(pcb, None);
        cpu.await_process_interrupt();
    
        let program_data = {
            let memory = memory.read().unwrap();
            let pcb = memory.get_pcb_for(0);
            let pcb = pcb.lock().unwrap();
            
            memory.read_block_from(0, pcb.get_mem_end_address())
        };
    
        let mut i = 0;
        for line in program_data {
            println!("{} {}",i, line);
            i += 1;
        }

        println!("\nRegisters:\n");
        i = 0;
        for value in cpu.resources.try_lock().unwrap().registers{
            println!("{} {}",i,value);
            i += 1;
        };

        println!("\nEnd CPU Test\n");
    }

    #[test]
    fn test_execute_job2() {

        println!("\nBegin CPU Test\n");

        let job2_program_info = ProgramInfo {
            id: 0,
            priority: 0,
            instruction_buffer_size: 28,
            in_buffer_size: 20,
            out_buffer_size: 12,
            temp_buffer_size: 12,
            data_start_idx: 28,
        };
    
        let job2_program_data:[u32;100] = [
            0xC0500070,
            0x4B060000,
            0x4B010000,
            0x4B000000,
            0x4F0A0070,
            0x4F0D00F0,
            0x4C0A0004,
            0xC0BA0000,
            0x42BD0000,
            0x4C0D0004,
            0x4C060001,
            0x10658000,
            0x56810018,
            0x4B060000,
            0x4F0900F0,
            0x43900000,
            0x4C060001,
            0x4C090004,
            0x43920000,
            0x4C060001,
            0x4C090004,
            0x10028000,
            0x55810060,
            0x04020000,
            0x10658000,
            0x56810048,
            0xC10000C0,
            0x92000000,
            // Data 14 C C
            0x0000000A,
            0x00000006,
            0x0000002C,
            0x00000045,
            0x00000001,
            0x00000007,
            0x00000000,
            0x00000001,
            0x00000005,
            0x0000000A,
            0x00000055,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
        ];

        println!("\nTesting job number 2: Find Max number");


        let mut memory = Memory::new();
    
        memory.create_process(&job2_program_info, &job2_program_data);
        let pcb = memory.get_pcb_for(0);
    
        let memory = Arc::new(RwLock::new(memory));
        let mut cpu = Cpu::new(memory.clone());
    
        cpu.execute_process(pcb, None);
        cpu.await_process_interrupt();
    
        let program_data = {
            let memory = memory.read().unwrap();
            let pcb = memory.get_pcb_for(0);
            let pcb = pcb.lock().unwrap();
            
            memory.read_block_from(0, pcb.get_mem_end_address())
        };
    
        let mut i = 0;
        for line in program_data {
            println!("{} {}",i, line);
            i += 1;
        }

        println!("\nRegisters:\n");
        i = 0;
        for value in cpu.resources.try_lock().unwrap().registers{
            println!("{} {}",i,value);
            i += 1;
        };

        println!("\nEnd CPU Test\n");
    }

    #[test]
    fn test_execute_job3() {

        println!("\nBegin CPU Test\n");


        let job3_program_info = ProgramInfo {
            id: 0,
            priority: 0,
            instruction_buffer_size: 24,
            in_buffer_size: 20,
            out_buffer_size: 12,
            temp_buffer_size: 12,
            data_start_idx: 24,
        };

        let job3_program_data:[u32;100] = [
            0xC0500060,
            0x4B060000,
            0x4B010000,
            0x4B000000,
            0x4F0A0060,
            0x4F0D00E0,
            0x4C0A0004,
            0xC0BA0000,
            0x42BD0000,
            0x4C0D0004,
            0x4C060001,
            0x10658000,
            0x56810018,
            0x4B060000,
            0x4F0900E0,
            0x43970000,
            0x05070000,
            0x4C060001,
            0x4C090004,
            0x10658000,
            0x5681003C,
            0x08050000,
            0xC10000B0,
            0x92000000,
            // Data 14 C C
            0x0000000A,
            0x00000006,
            0x0000002C,
            0x00000045,
            0x00000001,
            0x00000009,
            0x000000B0,
            0x00000001,
            0x00000007,
            0x000000AA,
            0x00000055,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
        ];

        println!("\nTesting job number 3: Average of inputs");


        let mut memory = Memory::new();
    
        memory.create_process(&job3_program_info, &job3_program_data);
        let pcb = memory.get_pcb_for(0);
    
        let memory = Arc::new(RwLock::new(memory));
        let mut cpu = Cpu::new(memory.clone());
    
        cpu.execute_process(pcb, None);
        cpu.await_process_interrupt();
    
        let program_data = {
            let memory = memory.read().unwrap();
            let pcb = memory.get_pcb_for(0);
            let pcb = pcb.lock().unwrap();
            
            memory.read_block_from(0, pcb.get_mem_end_address())
        };
    
        let mut i = 0;
        for line in program_data {
            println!("{} {}",i, line);
            i += 1;
        }

        println!("\nRegisters:\n");
        i = 0;
        for value in cpu.resources.try_lock().unwrap().registers{
            println!("{} {}",i,value);
            i += 1;
        };

        println!("\nEnd CPU Test\n");
    }

    #[test]
    fn test_execute_job4() {

        println!("\nBegin CPU Test\n");

        let job4_program_info = ProgramInfo {
            id: 0,
            priority: 0,
            instruction_buffer_size: 19,
            in_buffer_size: 20,
            out_buffer_size: 12,
            temp_buffer_size: 12,
            data_start_idx: 19,
        };
    
        let job4_program_data:[u32;100] = [
            0xC050004C,
            0x4B060000,
            0x4B000000,
            0x4B010000,
            0x4B020000,
            0x4B030001,
            0x4F07009C,
            0xC1270000,
            0x4C070004,
            0x4C060001,
            0x05320000,
            0xC1070000,
            0x4C070004,
            0x4C060001,
            0x04230000,
            0x04300000,
            0x10658000,
            0x56810028,
            0x92000000,
            // Data 14 C C
            0x0000000B,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
        ];

        println!("\nTesting job number 4: Fibonacci numbers");


        let mut memory = Memory::new();
    
        memory.create_process(&job4_program_info, &job4_program_data);
        let pcb = memory.get_pcb_for(0);
    
        let memory = Arc::new(RwLock::new(memory));
        let mut cpu = Cpu::new(memory.clone());
    
        cpu.execute_process(pcb, None);
        cpu.await_process_interrupt();
    
        let program_data = {
            let memory = memory.read().unwrap();
            let pcb = memory.get_pcb_for(0);
            let pcb = pcb.lock().unwrap();
            
            memory.read_block_from(0, pcb.get_mem_end_address())
        };
    
        let mut i = 0;
        for line in program_data {
            println!("{} {}",i, line);
            i += 1;
        }

        println!("\nRegisters:\n");
        i = 0;
        for value in cpu.resources.try_lock().unwrap().registers{
            println!("{} {}",i,value);
            i += 1;
        };

        println!("\nEnd CPU Test\n");
    }
}