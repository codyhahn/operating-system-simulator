use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex, RwLock};

use super::*;

use crate::io::{Disk, loader};

pub struct Driver {
    cpu: Arc<Mutex<Cpu>>,
    disk: Rc<RefCell<Disk>>,
    memory: Arc<RwLock<Memory>>,
    lts: LongTermScheduler,
    sts: ShortTermScheduler,
}

impl Driver {
    pub fn new() -> Driver {
        let disk = Rc::new(RefCell::new(Disk::new()));
        let memory = Arc::new(RwLock::new(Memory::new()));
        let cpu = Arc::new(Mutex::new(Cpu::new(memory.clone())));
        
        let disk_clone = disk.clone();
        let memory_clone = memory.clone();
        let cpu_clone = cpu.clone();

        Driver {
            cpu,
            disk,
            memory,
            lts: LongTermScheduler::new(disk_clone, memory_clone),
            sts: ShortTermScheduler::new(cpu_clone, StsSchedulingAlg::Fifo),
            // sts: ShortTermScheduler::new(cpu_clone, SchedulingAlg::Fifo),
        }
    }

    pub fn start(&mut self) {
        println!("Starting the driver.");
        println!("Loading programs into disk.");
        let program_ids = loader::load_programs_into_disk(&mut self.disk.borrow_mut())
            .unwrap_or_else(|err| {
                println!("Failed to load programs into disk: {}", err);
                return Vec::new();
            });

        if program_ids.is_empty() {
            println!("No programs to load into memory.");
            return;
        }   

        println!("Enqueuing programs into LTS.");
        self.lts.enqueue_programs(program_ids);

        println!("Starting the LTS.");
        while self.lts.has_programs() {
            let process_ids = self.lts.batch_step();
            let num_processes = process_ids.len();
            
            for process_id in process_ids {
                println!("Scheduling process {}.", process_id);
                let memory = self.memory.read().unwrap();
                let pcb = memory.get_pcb_for(process_id);
                self.sts.schedule_process(pcb);
            }

            println!("Dumped memory for {} processes.", num_processes);
            self.memory.write().unwrap().core_dump();
            // TODO: Implement core dump to file. Should do it on a separate thread. Method to do it should go in io/dump_writer.rs
        }
    }
}