use crate::io::ProgramInfo;

pub(crate) struct ProcessControlBlock {
    pub id: u32,
    pub priority: u32,

    pub mem_start_address: usize,
    pub mem_end_address: usize,
    pub program_counter: usize,
}

impl ProcessControlBlock {
    pub fn new(program_info: &ProgramInfo, mem_start_address: usize, mem_end_address: usize) -> ProcessControlBlock {
        ProcessControlBlock {
            id: program_info.id,
            priority: program_info.priority,
            mem_start_address,
            mem_end_address,
            program_counter: 0,
        }
    }
}