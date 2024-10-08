use std::cmp::Ordering;

use crate::io::ProgramInfo;

#[derive(Eq, PartialEq)]
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

impl Ord for ProcessControlBlock {
    fn cmp(&self, other: &Self) -> Ordering {
        self.priority.cmp(&other.priority)
    }
}

impl PartialOrd for ProcessControlBlock {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}