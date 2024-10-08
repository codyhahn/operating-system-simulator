use super::{Memory, LongTermScheduler, FifoQueue, PriorityQueue, ShortTermScheduler};

use crate::io::{Disk, loader};

pub struct Driver {
    disk: Disk,
    memory: Memory,
    lts: LongTermScheduler,
    sts: ShortTermScheduler,
}

impl Driver {
    pub fn new() -> Driver {
        Driver {
            disk: Disk::new(),
            memory: Memory::new(),
            lts: LongTermScheduler::new(),
            sts: ShortTermScheduler::new(Box::new(FifoQueue::new())),
            // sts: ShortTermScheduler::new(Box::new(PriorityQueue::new())),
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
        
        for process_id in process_ids {
            let pcb = self.memory.get_pcb_for(process_id);
            self.sts.schedule_process(pcb);
        }
    }
}