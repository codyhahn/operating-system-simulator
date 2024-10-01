use std::collections::HashMap;
use std::rc::Rc;

use super::Program;

const DISK_SIZE: usize = 4096;

pub struct Disk {
    program_map: HashMap<u32, Program>,
    data: [u32; DISK_SIZE],
    current_data_idx: usize
}

impl Disk {
    pub fn new() -> Disk {
        Disk {
            program_map: HashMap::new(),
            data: [0; DISK_SIZE],
            current_data_idx: 0
        }
    }

    pub fn read_program(&self, id: u32) -> &Program {
        match self.program_map.get(&id) {
            Some(program) => program,
            None => panic!("Program not found")
        }
    }

    pub fn write_program(&mut self,
                         id: u32,
                         priority: u32,
                         instruction_buffer_size: usize,
                         in_buffer_size: usize,
                         out_buffer_size: usize,
                         temp_buffer_size: usize,
                         data: &[u32]) {
        let start_data_idx = self.current_data_idx;
        let end_data_idx = start_data_idx + data.len();

        if end_data_idx > DISK_SIZE {
            panic!("Out of bounds disk access");
        }

        self.data[start_data_idx..end_data_idx].copy_from_slice(data);
        self.current_data_idx += data.len();

        let program = Program {
            id,
            priority,
            instruction_buffer_size,
            in_buffer_size,
            out_buffer_size,
            temp_buffer_size,
            data: Rc::from(&self.data[start_data_idx..end_data_idx])
        };
        
        self.program_map.insert(id, program);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_disk_write_then_read_program() {
        let mut disk = Disk::new();
        disk.write_program(0, 0, 0, 0, 0, 0, &[1, 2, 3, 4, 5]);

        let program = disk.read_program(0);
        assert_eq!(program.data.as_ref(), &[1, 2, 3, 4, 5]);
    }

    #[test]
    #[should_panic]
    fn test_disk_out_of_bounds_read_program() {
        let disk = Disk::new();
        disk.read_program(0);
    }

    #[test]
    #[should_panic]
    fn test_disk_out_of_bounds_write_program() {
        let mut disk = Disk::new();
        disk.write_program(0, 0, 0, 0, 0, 0, &[0; DISK_SIZE + 1]);
    }
}