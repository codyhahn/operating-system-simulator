use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex, RwLock};

use super::*;

use crate::io::{Disk, loader};

const SCHEDULING_ALG: StsSchedulingAlg = StsSchedulingAlg::Priority;

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
            sts: ShortTermScheduler::new(cpu_clone, SCHEDULING_ALG),
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

        println!("Enqueuing all programs into LTS.");
        self.lts.enqueue_programs(program_ids);

        let mut batch_num = 1;
        let mut process_stats = Vec::new();

        println!("Starting the LTS:");
        while self.lts.has_programs() {
            println!("...Batch {}:", batch_num);
            let process_ids = self.lts.batch_step();
            let num_processes = process_ids.len();

            println!("......Scheduling {} processes into STS.", num_processes);
            for process_id in process_ids {
                let memory = self.memory.read().unwrap();
                let pcb = memory.get_pcb_for(process_id);
                self.sts.schedule_process(pcb);
            }

            println!("......Awaiting all scheduled processes to finish.");
            self.sts.await_all_procs_finished();

            let pcbs = self.memory.read().unwrap().get_pcbs(true);
            for pcb in pcbs {
                let pcb = pcb.lock().unwrap();
                
                let id = pcb.get_id();
                let priority = pcb.get_priority();
                let turnaround_time_ms = pcb.get_turnaround_time_ms();
                let avg_burst_time_ms = pcb.get_avg_burst_time_ms();

                process_stats.push((id, priority, turnaround_time_ms, avg_burst_time_ms));
            }

            // TODO: Update disk using memory before dumping.

            println!("......Dumping memory for {} processes.", num_processes);
            self.memory.write().unwrap().core_dump();

            batch_num += 1;
        }

        print!("Stats for executed processes (");
        match SCHEDULING_ALG {
            StsSchedulingAlg::Fifo => println!("FIFO Scheduling):"),
            StsSchedulingAlg::Priority => println!("Priority Scheduling):"),
        }
        println!("... ID | Priority | Turnaround Time (ms) | Avg Burst Time (ms)");
        println!("...----|----------|----------------------|---------------------");
        for (id, priority, turnaround_time_ms, avg_burst_time_ms) in process_stats {
            println!(
                "... {:02} | {:02}       | {:05.2}                | {:05.2}",
                id,
                priority,
                turnaround_time_ms,
                avg_burst_time_ms
            );
        }

        // TODO: Implement writing disk to file. Should be same format as program_file.txt. Make a module in io for it.
    }
}