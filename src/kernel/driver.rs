use std::cell::RefCell;
use std::rc::Rc;

use super::Memory;
use super::LongTermScheduler;

use crate::io::Disk;
use crate::io::loader;

pub struct Driver {
    disk: Disk,
    memory: Memory,
    lts: LongTermScheduler
}

impl Driver {
    pub fn new() -> Driver {
        Driver {
            disk: Disk::new(),
            memory: Memory::new(),
            lts: LongTermScheduler::new()
        }
    }

    pub fn start(&mut self) {
        if loader::load_programs_into_disk(&mut self.disk).is_ok() {
            println!("Programs loaded into disk successfully");
        } else {
            println!("Failed to load programs into disk");
        }

        let process_ids = self.lts.batch_step(&mut self.disk, &mut self.memory);
        
        // TODO: Get PCB refs from memory and submit them to ShortTermScheduler
    }
}