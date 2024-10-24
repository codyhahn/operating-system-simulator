use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::ProcessControlBlock;

use crate::io::{Disk, ProgramInfo};
use crate::io::disk;


const MEMORY_SIZE: usize = 1024;

pub(crate) struct Memory {
    pcb_map: HashMap<u32, Arc<Mutex<ProcessControlBlock>>>,
    data: [u32; MEMORY_SIZE],
    current_data_idx: usize,
}

impl Memory {
    pub fn new() -> Memory {
        Memory {
            pcb_map: HashMap::new(),
            data: [0; MEMORY_SIZE],
            current_data_idx: 0,
        }
    }

    pub fn read_from(&self, address: usize) -> u32 {
        if address >= MEMORY_SIZE {
            panic!("Out of bounds memory access. Address is greater than memory size");
        }

        self.data[address]
    }

    pub fn read_block_from(&self, start_address: usize, end_address: usize) -> Vec<u32> {
        if start_address >= MEMORY_SIZE || end_address >= MEMORY_SIZE {
            panic!("Out of bounds memory access. Start or end address is greater than memory size");
        } else if start_address > end_address {
            panic!("Invalid memory range. Start address is greater than end address");
        }

        self.data[start_address..end_address].to_vec()
    }

    pub fn write_to(&mut self, address: usize, value: u32) {
        if address >= MEMORY_SIZE {
            panic!("Out of bounds memory access");
        }

        self.data[address] = value;
    }

    pub fn write_block_to(&mut self, address: usize, data: &[u32]) {
        let start_address = address;
        let end_address = address + data.len();

        if end_address > MEMORY_SIZE {
            panic!("Out of bounds memory access");
        }

        self.data[start_address..end_address].copy_from_slice(data);
    }

    pub fn create_process(&mut self, program_info: &ProgramInfo, program_data: &[u32]) {
        let start_address = self.current_data_idx;
        let end_address = start_address + program_data.len();
        self.current_data_idx = end_address;

        self.write_block_to(start_address, program_data);

        let pcb = Arc::from(Mutex::new(ProcessControlBlock::new(program_info, start_address, end_address)));
        pcb.lock().unwrap().start_record_turnaround_time();
        self.pcb_map.insert(program_info.id, pcb);
    }

    pub fn get_pcb_for(&self, process_id: u32) -> Arc<Mutex<ProcessControlBlock>> {
        match self.pcb_map.get(&process_id) {
            Some(pcb) => pcb.clone(),
            _ => panic!("No process found for id: {}", process_id)
        }
    }

    pub fn get_pcbs(&self, should_sort: bool) -> Vec<Arc<Mutex<ProcessControlBlock>>> {
        if should_sort {
            let mut pcbs = self.get_pcbs(false);
            pcbs.sort_by(|a, b| a.lock().unwrap().get_id().cmp(&b.lock().unwrap().get_id()));
            pcbs
        } else {
            self.pcb_map.values().cloned().collect()
        }
    }

    pub fn core_dump(&mut self) {
        // TODO: Implement writing mem to file.
        let disk = self.data;
        //let disk = self.pcb_map;
        self.pcb_map.clear();
        let empty_data = [0; MEMORY_SIZE];
        self.write_block_to(0, &empty_data);
        self.current_data_idx = 0;
    }

    pub fn get_remaining_memory(&self) -> usize {
        MEMORY_SIZE - self.current_data_idx
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_read_from() {
        let memory = Memory::new();
        assert_eq!(memory.read_from(0), 0);
    }

    #[test]
    #[should_panic]
    fn test_memory_out_of_bounds_read_from() {
        let memory = Memory::new();
        memory.read_from(1024);
    }

    #[test]
    fn test_memory_write_to() {
        let mut memory = Memory::new();
        memory.write_to(0, 10);
        assert_eq!(memory.read_from(0), 10);
    }

    #[test]
    #[should_panic]
    fn test_memory_out_of_bounds_write_to() {
        let mut memory = Memory::new();
        memory.write_to(1024, 10);
    }

    #[test]
    fn test_memory_read_block_from() {
        let memory = Memory::new();
        let block = memory.read_block_from(0, 5);
        assert_eq!(block, &[0, 0, 0, 0, 0]);
    }

    #[test]
    #[should_panic]
    fn test_memory_out_of_bounds_read_block_from() {
        let memory = Memory::new();
        memory.read_block_from(0, 1024);
    }

    #[test]
    #[should_panic]
    fn test_memory_invalid_range_read_block_from() {
        let memory = Memory::new();
        memory.read_block_from(5, 0);
    }

    #[test]
    fn test_memory_write_block_to() {
        let mut memory = Memory::new();
        let block = [1, 2, 3, 4, 5];
        memory.write_block_to(0, &block);
        let block = memory.read_block_from(0, 5);
        assert_eq!(block, &[1, 2, 3, 4, 5]);
    }

    #[test]
    #[should_panic]
    fn test_memory_out_of_bounds_write_block_to() {
        let mut memory = Memory::new();
        let block = [1, 2, 3, 4, 5];
        memory.write_block_to(1020, &block);
    }

    #[test]
    fn test_memory_create_process_then_get_pcb_for() {
        let mut memory = Memory::new();
        let program_info = ProgramInfo {
            id: 1,
            priority: 1,
            instruction_buffer_size: 1,
            in_buffer_size: 1,
            out_buffer_size: 1,
            temp_buffer_size: 2,
            data_start_idx: 0
        };
        let program_data = [1, 2, 3, 4, 5];
        memory.create_process(&program_info, &program_data);
        let binding = memory.get_pcb_for(1);
        let pcb = binding.lock().unwrap();
        assert_eq!(pcb.get_id(), 1);
        assert_eq!(pcb.get_priority(), 1);
        assert_eq!(pcb.get_mem_start_address(), 0);
        assert_eq!(pcb.get_mem_end_address(), 5);
    }

    #[test]
    #[should_panic]
    fn test_memory_get_pcb_for_invalid_id() {
        let memory = Memory::new();
        memory.get_pcb_for(1);
    }

    #[test]
    fn test_memory_core_dump() {
        let mut disk = Disk::new();
        let mut memory = Memory::new();
        let program_info = ProgramInfo {
            id: 1,
            priority: 1,
            instruction_buffer_size: 1,
            in_buffer_size: 1,
            out_buffer_size: 1,
            temp_buffer_size: 2,
            data_start_idx: 0
        };
        disk.write_program(2,2,2,2,2,3, &[5,6,7,8,9]);
        let program_data = [1, 2, 3, 4, 5];
        memory.create_process(&program_info, &program_data);
        memory.core_dump(&disk);
        assert_eq!(memory.pcb_map.len(), 0);
        assert_eq!(memory.read_from(0), 0);
    }

    #[test]
    fn test_memory_get_remaining_memory() {
        let mut memory = Memory::new();
        let program_info = ProgramInfo {
            id: 1,
            priority: 1,
            instruction_buffer_size: 1,
            in_buffer_size: 1,
            out_buffer_size: 1,
            temp_buffer_size: 2,
            data_start_idx: 0
        };
        let program_data = [1, 2, 3, 4, 5];
        memory.create_process(&program_info, &program_data);
        assert_eq!(memory.get_remaining_memory(), 1019);
    }
}