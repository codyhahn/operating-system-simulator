use crate::io::ProgramInfo;

#[derive(Clone, Copy)]
pub(crate) enum ProcessState {
    Ready,
    Running,
    Waiting,
    Terminated,
}

pub(crate) struct ProcessControlBlock {
    pub program_counter: usize,
    pub registers: [u32; 16],
    pub state: ProcessState,
    
    id: u32,
    priority: u32,

    mem_start_address: usize,
    mem_in_start_address: usize,
    mem_out_start_address: usize,
    mem_temp_start_address: usize,
    mem_end_address: usize,
    
}

impl ProcessControlBlock {
    pub fn new(program_info: &ProgramInfo, mem_start_address: usize, mem_end_address: usize) -> ProcessControlBlock {
        ProcessControlBlock {
            id: program_info.id,
            priority: program_info.priority,
            mem_start_address,
            mem_in_start_address: mem_start_address + program_info.instruction_buffer_size,
            mem_out_start_address: mem_start_address + program_info.instruction_buffer_size + program_info.in_buffer_size,
            mem_temp_start_address: mem_start_address + program_info.instruction_buffer_size + program_info.in_buffer_size + program_info.out_buffer_size,
            mem_end_address,
            program_counter: 0,
            registers: [0; 16],
            state: ProcessState::Ready,
        }
    }

    pub fn get_id(&self) -> u32 {
        self.id
    }

    pub fn get_priority(&self) -> u32 {
        self.priority
    }

    pub fn get_mem_start_address(&self) -> usize {
        self.mem_start_address
    }

    pub fn get_mem_in_start_address(&self) -> usize {
        self.mem_in_start_address
    }

    pub fn get_mem_out_start_address(&self) -> usize {
        self.mem_out_start_address
    }

    pub fn get_mem_temp_start_address(&self) -> usize {
        self.mem_temp_start_address
    }

    pub fn get_mem_end_address(&self) -> usize {
        self.mem_end_address
    }
}