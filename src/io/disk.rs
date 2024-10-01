use std::collections::HashMap;

use super::ProgramInfo;

const DISK_SIZE: usize = 4096;

pub struct Disk {
    program_map: HashMap<u32, ProgramInfo>,
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

    pub fn get_info_for(&self, program_id: u32) -> &ProgramInfo {
        match self.program_map.get(&program_id) {
            Some(program_info) => program_info,
            None => panic!("Program not found")
        }
    }
    pub fn read_data_for(&self, program_info: &ProgramInfo) -> &[u32] {
        let data_start_idx = program_info.data_start_idx;
        let data_end_idx = data_start_idx + program_info.instruction_buffer_size +
            program_info.in_buffer_size + program_info.out_buffer_size + program_info.temp_buffer_size;

        &self.data[data_start_idx..data_end_idx]
    }

    pub fn write_program(&mut self,
                         id: u32,
                         priority: u32,
                         instruction_buffer_size: usize,
                         in_buffer_size: usize,
                         out_buffer_size: usize,
                         temp_buffer_size: usize,
                         data: &[u32]) {
        let data_start_idx = self.current_data_idx;
        let data_end_idx = data_start_idx + data.len();

        if data_end_idx > DISK_SIZE {
            panic!("Out of bounds disk access");
        }

        self.data[data_start_idx..data_end_idx].copy_from_slice(data);
        self.current_data_idx += data.len();

        let program_info = ProgramInfo {
            id,
            priority,
            instruction_buffer_size,
            in_buffer_size,
            out_buffer_size,
            temp_buffer_size,
            data_start_idx
        };
        
        self.program_map.insert(id, program_info);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_disk_write_program_then_read_data_for() {
        let mut disk = Disk::new();
        disk.write_program(0, 0, 1, 1, 1, 2, &[1, 2, 3, 4, 5]);

        let data = disk.read_data_for(disk.get_info_for(0));
        assert_eq!(data, &[1, 2, 3, 4, 5]);
    }

    #[test]
    #[should_panic]
    fn test_disk_out_of_bounds_read_data_for() {
        let disk = Disk::new();
        disk.get_info_for(0);
    }

    #[test]
    #[should_panic]
    fn test_disk_out_of_bounds_write_program() {
        let mut disk = Disk::new();
        disk.write_program(0, 0, 0, 0, 0, 0, &[0; DISK_SIZE + 1]);
    }
}