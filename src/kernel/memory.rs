use std::sync::RwLock;

const MEMORY_SIZE: usize = 1024;

pub(crate) struct Memory {
    data: RwLock<[u32; MEMORY_SIZE]>
}

impl Memory {
    pub fn new() -> Memory {
        Memory {
            data: RwLock::new([0; MEMORY_SIZE])
        }
    }

    pub fn read_from(&self, address: usize) -> u32 {
        if address >= MEMORY_SIZE {
            panic!("Out of bounds memory access. Address is greater than memory size");
        }

        self.data.read().unwrap()[address]
    }

    pub fn read_block_from(&self, start_address: usize, end_address: usize) -> Vec<u32> {
        if start_address >= MEMORY_SIZE || end_address >= MEMORY_SIZE {
            panic!("Out of bounds memory access. Start or end address is greater than memory size");
        } else if start_address > end_address {
            panic!("Invalid memory range. Start address is greater than end address");
        }

        self.data.read().unwrap()[start_address..end_address].to_vec()
    }

    pub fn write_to(&mut self, address: usize, value: u32) {
        if address >= MEMORY_SIZE {
            panic!("Out of bounds memory access");
        }

        self.data.write().unwrap()[address] = value;
    }

    pub fn write_block_to(&mut self, address: usize, data: &[u32]) {
        let start_address = address;
        let end_address = address + data.len();

        if end_address > MEMORY_SIZE {
            panic!("Out of bounds memory access");
        }

        self.data.write().unwrap()[start_address..end_address].copy_from_slice(data);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_read() {
        let memory = Memory::new();
        assert_eq!(memory.read_from(0), 0);
    }

    #[test]
    #[should_panic]
    fn test_memory_out_of_bounds_read() {
        let memory = Memory::new();
        memory.read_from(1024);
    }

    #[test]
    fn test_memory_write() {
        let mut memory = Memory::new();
        memory.write_to(0, 10);
        assert_eq!(memory.read_from(0), 10);
    }

    #[test]
    #[should_panic]
    fn test_memory_out_of_bounds_write() {
        let mut memory = Memory::new();
        memory.write_to(1024, 10);
    }

    #[test]
    fn test_memory_read_block() {
        let memory = Memory::new();
        let block = memory.read_block_from(0, 5);
        assert_eq!(block, &[0, 0, 0, 0, 0]);
    }

    #[test]
    #[should_panic]
    fn test_memory_out_of_bounds_read_block() {
        let memory = Memory::new();
        memory.read_block_from(0, 1024);
    }

    #[test]
    #[should_panic]
    fn test_memory_invalid_range_read_block() {
        let memory = Memory::new();
        memory.read_block_from(5, 0);
    }

    #[test]
    fn test_memory_write_block() {
        let mut memory = Memory::new();
        let block = [1, 2, 3, 4, 5];
        memory.write_block_to(0, &block);
        let block = memory.read_block_from(0, 5);
        assert_eq!(block, &[1, 2, 3, 4, 5]);
    }

    #[test]
    #[should_panic]
    fn test_memory_out_of_bounds_write_block() {
        let mut memory = Memory::new();
        let block = [1, 2, 3, 4, 5];
        memory.write_block_to(1020, &block);
    }
}