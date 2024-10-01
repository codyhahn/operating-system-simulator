use std::cell::RefCell;
use std::rc::Rc;

use super::Memory;
use super::LongTermScheduler;

use crate::io::Disk;
use crate::io::loader;

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
        let program_ids = loader::load_programs_into_disk(&mut self.disk)
            .unwrap_or_else(|err| {
                println!("Failed to load programs into disk: {}", err);
                return Vec::new();
            });

        if program_ids.is_empty() {
            println!("No programs to load into memory.");
            return;
        }

        self.lts.enqueue_programs(program_ids);
        let process_ids = self.lts.batch_step(&mut self.disk, &mut self.memory);
        
        // TODO: Get PCB refs from memory and submit them to ShortTermScheduler
    }
}