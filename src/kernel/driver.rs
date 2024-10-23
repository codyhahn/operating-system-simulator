use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex, RwLock};

use super::*;

use crate::io::{Disk, loader};
use crate::io::disk;

pub struct Driver {
    _cpu: Arc<Mutex<Cpu>>,
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
            _cpu: cpu,
            disk,
            memory,
            lts: LongTermScheduler::new(disk_clone, memory_clone),
            sts: ShortTermScheduler::new(cpu_clone, StsSchedulingAlg::Fifo),
            // sts: ShortTermScheduler::new(cpu_clone, StsSchedulingAlg::Priority),
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

            println!("Awaiting all processes to finish.");
            self.sts.await_all_procs_finished();

            println!("Dumped memory for {} processes after completion.", num_processes);
            self.memory.write().unwrap().core_dump(self.disk);
            // TODO: Update disk using contents of dumped memory.
        }

        // TODO: Implement writing disk to file. Should be same format as program_file.txt. Make a module in io for it.
    }
}