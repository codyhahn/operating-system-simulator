use std::collections::VecDeque;

use super::Memory;

use crate::io::Disk;

const PROGRAM_IDS: [u32; 30] = [
    1, 2, 3, 4, 5, 6, 7, 8, 9, 10,
    11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
    21, 22, 23, 24, 25, 26, 27, 28, 29, 30,
];

pub(crate) struct LongTermScheduler {
    program_queue: VecDeque<u32>
}

impl LongTermScheduler {
    pub fn new() -> LongTermScheduler {
        LongTermScheduler {
            program_queue: VecDeque::from(PROGRAM_IDS)
        }
    }

    pub fn step(&mut self, disk: &mut Disk, memory: &mut Memory) -> Result<u32, &'static str> {
        let program_id = *self.program_queue.front().ok_or("No programs in queue")?;
        
        let program_info = disk.get_info_for(program_id);
        let program_data = disk.read_data_for(&program_info);

        if memory.get_remaining_memory() < program_data.len() {
            return Err("Not enough memory to load program");
        }

        self.program_queue.pop_front();
        memory.create_process(program_info, program_data);

        Ok(program_id)
    }

    pub fn batch_step(&mut self, disk: &mut Disk, memory: &mut Memory) -> Vec<u32> {
        let mut process_ids = Vec::new();

        while !self.program_queue.is_empty() {
            match self.step(disk, memory) {
                Ok(process_id) => process_ids.push(process_id),
                Err(_) => break
            }
        }

        process_ids
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_long_term_scheduler_step() {
        let mut lts = LongTermScheduler::new();
        let mut disk = Disk::new();
        let mut memory = Memory::new();

        disk.write_program(1, 1, 1, 1, 1, 2, &[1, 2, 3, 4, 5]);
        let process_id = lts.step(&mut disk, &mut memory).unwrap();

        assert_eq!(process_id, 1);

        disk.write_program(2, 1, 1, 1, 1, 2, &[1, 2, 3, 4, 5]);
        let process_id = lts.step(&mut disk, &mut memory).unwrap();

        assert_eq!(process_id, 2);
    }
}
