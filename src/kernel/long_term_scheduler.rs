use std::collections::VecDeque;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, RwLock};

use super::Memory;

use crate::io::Disk;

pub(crate) struct LongTermScheduler {
    disk: Rc<RefCell<Disk>>,
    memory: Arc<RwLock<Memory>>,
    program_queue: VecDeque<u32>,
}

impl LongTermScheduler {
    pub fn new(disk: Rc<RefCell<Disk>>, memory: Arc<RwLock<Memory>>) -> LongTermScheduler {
        LongTermScheduler {
            disk,
            memory,
            program_queue: VecDeque::new(),
        }
    }

    pub fn enqueue_programs(&mut self, program_ids: Vec<u32>) {
        self.program_queue.extend(program_ids);
    }

    pub fn step(&mut self) -> Result<u32, &'static str> {
        let program_id = *self.program_queue.front().ok_or("No programs in queue")?;
        let disk = self.disk.borrow();

        let program_info = disk.get_info_for(program_id);
        let program_data = disk.read_data_for(&program_info);

        let mut memory = self.memory.write().unwrap();

        if memory.get_remaining_memory() < program_data.len() {
            return Err("Not enough memory to load program");
        }

        self.program_queue.pop_front();
        memory.create_process(program_info, program_data);

        Ok(program_id)
    }

    pub fn batch_step(&mut self) -> Vec<u32> {
        let mut process_ids = Vec::new();

        while !self.program_queue.is_empty() {
            match self.step() {
                Ok(process_id) => process_ids.push(process_id),
                Err(_) => break
            }
        }

        process_ids
    }

    pub fn unload_all(&self){
        let mut ids = Vec::new();
        self.disk.data = self.memory.data;

    }

    pub fn has_programs(&self) -> bool {
        !self.program_queue.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use std::vec;

    use super::*;

    #[test]
    fn test_long_term_scheduler_enqueue_then_step() {
        let mut disk = Disk::new();

        disk.write_program(20, 1, 1, 1, 1, 2, &[1, 2, 3, 4, 5]);
        
        let disk = Rc::new(RefCell::new(disk));
        let memory = Arc::new(RwLock::new(Memory::new()));
        let mut lts = LongTermScheduler::new(disk, memory);

        lts.enqueue_programs(vec![20]);
        let process_id = lts.step().unwrap();

        assert_eq!(process_id, 20);
    }

    #[test]
    fn test_long_term_scheduler_enqueue_then_batch_step() {
        let mut disk = Disk::new();

        disk.write_program(20, 1, 1, 1, 1, 2, &[1, 2, 3, 4, 5]);
        disk.write_program(21, 1, 1, 1, 1, 2, &[1, 2, 3, 4, 5]);
        
        let disk = Rc::new(RefCell::new(disk));
        let memory = Arc::new(RwLock::new(Memory::new()));
        let mut lts = LongTermScheduler::new(disk, memory);

        lts.enqueue_programs(vec![20, 21]);
        let process_ids = lts.batch_step();

        assert_eq!(process_ids, vec![20, 21]);
    }

    #[test]
    fn test_long_term_scheduler_step_not_enough_memory() {
        let mut disk = Disk::new();
        let memory = Memory::new();

        let program_data = vec![1; memory.get_remaining_memory() - 1];
        
        disk.write_program(1, 1, program_data.len() - 3, 1, 1, 1, &program_data.as_slice());
        disk.write_program(2, 1, 1, 1, 1, 2, &[1, 2, 3, 4, 5]);
        
        let disk = Rc::new(RefCell::new(disk));
        let memory = Arc::new(RwLock::new(memory));
        let mut lts = LongTermScheduler::new(disk, memory);
        
        lts.enqueue_programs(vec![1, 2]);
        let _ = lts.step();
        let result = lts.step();

        assert_eq!(result, Err("Not enough memory to load program"));
    }

    #[test]
    fn test_long_term_scheduler_step_no_programs_in_queue() {
        let disk = Rc::new(RefCell::new(Disk::new()));
        let memory = Arc::new(RwLock::new(Memory::new()));
        let mut lts = LongTermScheduler::new(disk.clone(), memory.clone());

        let result = lts.step();

        assert_eq!(result, Err("No programs in queue"));
    }

    #[test]
    fn test_long_term_scheduler_batch_step_not_enough_memory() {
        let mut disk = Disk::new();
        let memory = Memory::new();

        let program_data = vec![1; memory.get_remaining_memory() - 1];

        disk.write_program(1, 1, program_data.len() - 3, 1, 1, 1, &program_data.as_slice());
        disk.write_program(2, 1, 1, 1, 1, 2, &[1, 2, 3, 4, 5]);

        let disk = Rc::new(RefCell::new(disk));
        let memory = Arc::new(RwLock::new(memory));
        let mut lts = LongTermScheduler::new(disk, memory.clone());

        lts.enqueue_programs(vec![1, 2]);
        let process_ids = lts.batch_step();

        assert_eq!(process_ids, vec![1]);

        memory.write().unwrap().core_dump(&disk.borrow_mut());
        let process_ids = lts.batch_step();

        assert_eq!(process_ids, vec![2]);
    }
}
