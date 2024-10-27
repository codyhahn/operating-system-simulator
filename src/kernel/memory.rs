use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::ProcessControlBlock;

use crate::io::ProgramInfo;

const MEMORY_SIZE: usize = 1024;

/// Simulated memory for storing program data.
/// 
/// The memory is used to store process data for the simulated operating system. The
/// memory is an array of 32-bit words. The memory is divided into sections for each
/// process where each process is stored in a contiguous block of memory. The memory also
/// stores a process control block (PCB) for each process in a hashmap. The PCB contains 
/// information about the process such as the process ID, priority, and memory address 
/// range. The PCB also stores registers, program counter, turnaround time, and 
/// burst time information of the process.
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

    /// Reads a 32-bit word from the memory at the specified address.
    /// 
    /// # Parameters
    /// 
    /// * `address` - The memory address to read from.
    /// 
    /// # Panics
    /// 
    /// Panics if the address is greater than or equal to the memory size.
    /// 
    /// # Returns
    /// 
    /// The 32-bit word read from memory.
    pub fn read_from(&self, address: usize) -> u32 {
        if address >= MEMORY_SIZE {
            panic!("Out of bounds memory access. Address is greater than memory size");
        }

        self.data[address]
    }

    /// Reads a block of 32-bit words from the memory at the specified address range.
    /// 
    /// # Parameters
    /// 
    /// * `start_address` - The starting memory address to read from.
    /// * `end_address` - The ending memory address to read from.
    /// 
    /// # Panics
    /// 
    /// Panics if the start or end address is greater than or equal to the memory size
    /// or if the start address is greater than the end address.
    /// 
    /// # Returns
    /// 
    /// A vector of 32-bit words read from memory.
    pub fn read_block_from(&self, start_address: usize, end_address: usize) -> Vec<u32> {
        if start_address >= MEMORY_SIZE || end_address >= MEMORY_SIZE {
            panic!("Out of bounds memory access. Start or end address is greater than memory size");
        } else if start_address > end_address {
            panic!("Invalid memory range. Start address is greater than end address");
        }

        self.data[start_address..end_address].to_vec()
    }

    /// Writes a 32-bit word to the memory at the specified address.
    /// 
    /// # Parameters
    /// 
    /// * `address` - The memory address to write to.
    /// * `value` - The 32-bit word to write to memory.
    /// 
    /// # Panics
    /// 
    /// Panics if the address is greater than or equal to the memory size.
    pub fn write_to(&mut self, address: usize, value: u32) {
        if address >= MEMORY_SIZE {
            panic!("Out of bounds memory access");
        }

        self.data[address] = value;
    }

    /// Writes a block of 32-bit words to the memory at the specified address range.
    /// 
    /// # Parameters
    /// 
    /// * `address` - The starting memory address to write to.
    /// * `data` - The vector of 32-bit words to write to memory.
    /// 
    /// # Panics
    /// 
    /// Panics if the data length exceeds the remaining memory size
    /// based on the start address.
    pub fn write_block_to(&mut self, address: usize, data: &[u32]) {
        let start_address = address;
        let end_address = address + data.len();

        if end_address > MEMORY_SIZE {
            panic!("Out of bounds memory access");
        }

        self.data[start_address..end_address].copy_from_slice(data);
    }

    /// Creates a process (PCB) in memory with the specified program information and 
    /// program data.
    /// 
    /// The program data is written to memory at the next available memory address. The
    /// ProcessControlBlock (PCB) is created with the program information and memory
    /// address range. The PCB is stored in a HashMap with the process ID as the key.
    /// 
    /// # Parameters
    /// 
    /// * `program_info` - The program information for the process.
    /// * `program_data` - The program data to write to memory.
    /// 
    /// # Panics
    /// 
    /// Panics if the program data length exceeds the remaining memory size.
    pub fn create_process(&mut self, program_info: &ProgramInfo, program_data: &[u32]) {
        let start_address = self.current_data_idx;
        let end_address = start_address + program_data.len();
        self.current_data_idx = end_address;

        self.write_block_to(start_address, program_data);

        let pcb = Arc::from(Mutex::new(ProcessControlBlock::new(program_info, start_address, end_address)));
        pcb.lock().unwrap().start_record_turnaround_time(); // Start recording turnaround time.
        self.pcb_map.insert(program_info.id, pcb);
    }

    /// Gets the process control block (PCB) for the specified process ID.
    /// 
    /// # Parameters
    /// 
    /// * `process_id` - The ID of the process to get the PCB for.
    /// 
    /// # Panics
    /// 
    /// Panics if no process is found for the specified process ID.
    /// 
    /// # Returns
    /// 
    /// The process control block (PCB) for the specified process ID.
    pub fn get_pcb_for(&self, process_id: u32) -> Arc<Mutex<ProcessControlBlock>> {
        match self.pcb_map.get(&process_id) {
            Some(pcb) => pcb.clone(),
            _ => panic!("No process found for id: {}", process_id)
        }
    }

    /// Gets all the process control blocks (PCBs) in memory.
    /// 
    /// # Parameters
    /// 
    /// * `should_sort` - A flag indicating whether to sort the PCBs by ID in ascending order.
    /// 
    /// # Returns
    /// 
    /// A vector of all the process control blocks (PCBs) in memory.
    pub fn get_pcbs(&self, should_sort: bool) -> Vec<Arc<Mutex<ProcessControlBlock>>> {
        if should_sort {
            let mut pcbs = self.get_pcbs(false);
            pcbs.sort_by(|a, b| a.lock().unwrap().get_id().cmp(&b.lock().unwrap().get_id()));
            pcbs
        } else {
            self.pcb_map.values().cloned().collect()
        }
    }

    /// Dumps the memory and clears all processes (PCBs).
    pub fn core_dump(&mut self) {
        self.pcb_map.clear();
        let empty_data = [0; MEMORY_SIZE];
        self.write_block_to(0, &empty_data);
        self.current_data_idx = 0;
    }

    /// Gets the remaining memory size available for storing process data.
    /// 
    /// # Returns
    /// 
    /// The remaining memory size available for storing process data.
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
    fn test_get_pcbs_sorted() {
        let mut memory = Memory::new();
        let program_info1 = ProgramInfo {
            id: 30,
            priority: 1,
            instruction_buffer_size: 1,
            in_buffer_size: 1,
            out_buffer_size: 1,
            temp_buffer_size: 2,
            data_start_idx: 0
        };
        let program_data1 = [1, 2, 3, 4, 5];
        memory.create_process(&program_info1, &program_data1);

        let program_info2 = ProgramInfo {
            id: 2,
            priority: 1,
            instruction_buffer_size: 1,
            in_buffer_size: 1,
            out_buffer_size: 1,
            temp_buffer_size: 2,
            data_start_idx: 5
        };
        let program_data2 = [1, 2, 3, 4, 5];
        memory.create_process(&program_info2, &program_data2);

        let pcbs = memory.get_pcbs(true);
        assert_eq!(pcbs[0].lock().unwrap().get_id(), 2);
        assert_eq!(pcbs[1].lock().unwrap().get_id(), 30);
    }

    #[test]
    fn test_memory_core_dump() {
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
        memory.core_dump();
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