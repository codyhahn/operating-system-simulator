use std::collections::{BinaryHeap, VecDeque};
use std::sync::{Arc, Condvar, Mutex, atomic::{AtomicBool, Ordering}};
use std::thread;

use super::{Cpu, ProcessControlBlock, ProcessState};

#[allow(dead_code)]
pub(crate) enum StsSchedulingAlg {
    Fifo,
    Priority,
}

/// The short-term scheduler is responsible for scheduling processes on the CPU.
/// 
/// The short-term scheduler has a ready queue that holds processes that are ready
/// to be executed on the CPU. The short-term scheduler uses a scheduling algorithm
/// to determine the order in which processes are scheduled on the CPU.
/// 
/// The short-term scheduler runs in a separate thread and dispatches processes
/// as they appear in the ready queue to the CPU as the CPU becomes available. 
/// The short-term scheduler also provides a way to wait until all processes
/// have finished executing.
pub(crate) struct ShortTermScheduler {
    resources: Arc<Mutex<ShortTermSchedulerResources>>,
    dispatch_should_terminate: Arc<AtomicBool>,
}

impl ShortTermScheduler {
    pub fn new(cpu: Arc<Mutex<Cpu>>, scheduling_alg: StsSchedulingAlg) -> ShortTermScheduler {
        let ready_queue: Box<dyn SchedulerQueue + Send> = match scheduling_alg {
            StsSchedulingAlg::Fifo => Box::new(FifoQueue::new()),
            StsSchedulingAlg::Priority => Box::new(PriorityQueue::new()),
        };

        let resources = Arc::new(Mutex::new(ShortTermSchedulerResources::new(
            cpu,
            ready_queue,
        )));
        let dispatch_should_terminate = Arc::new(AtomicBool::new(false));

        let resources_clone = resources.clone();
        let dispatch_should_terminate_clone = dispatch_should_terminate.clone();

        // Dispatch thread.
        thread::spawn(move || {
            while !dispatch_should_terminate_clone.load(Ordering::Relaxed) {
                ShortTermScheduler::dispatch(&resources_clone);
            }
        });

        ShortTermScheduler {
            resources,
            dispatch_should_terminate,
        }
    }

    /// Schedules a process to be executed on the CPU.
    /// 
    /// The process is added to the ready queue and will be executed by the CPU
    /// when it becomes available.
    /// 
    /// # Parameters
    /// 
    /// * `pcb` - The process control block to schedule.  
    pub fn schedule_process(&mut self, pcb: Arc<Mutex<ProcessControlBlock>>) {
        let mut resources = self.resources.lock().unwrap();
        resources.ready_queue.push(pcb);

        // Notify the dispatch thread that a new process is ready to be scheduled.
        let (lock, condvar) = &*resources.all_procs_are_finished_condvar;
        let mut all_procs_are_finished = lock.lock().unwrap();

        *all_procs_are_finished = false;
        condvar.notify_all();
    }

    /// Awaits all processes to finish executing.
    /// 
    /// This method blocks until all processes have finished executing.
    pub fn await_all_procs_finished(&self) {
        let all_procs_are_finished_condvar = {
            let resources = self.resources.lock().unwrap();
            resources.all_procs_are_finished_condvar.clone()
        };
        
        let (lock, condvar) = &*all_procs_are_finished_condvar;
        let mut all_procs_are_finished = lock.lock().unwrap();

        while !*all_procs_are_finished {
            all_procs_are_finished = condvar.wait(all_procs_are_finished).unwrap();
        }
    }

    /// Dispatches a process to the CPU.
    /// 
    /// This method is called by the dispatch thread to dispatch a process to the CPU.
    /// The method blocks until there are processes in the ready queue. The method
    /// will dispatch the process to the CPU and wait for the process to finish executing.
    /// 
    /// If there are no more processes to execute, the method will notify the main thread
    /// that all processes have finished executing.
    /// 
    /// # Parameters
    /// 
    /// * `resources` - The resources needed to dispatch a process.
    /// 
    /// # Panics
    /// 
    /// This method will panic if the process state is set to running after being moved out
    /// of the CPU.
    fn dispatch(resources: &Arc<Mutex<ShortTermSchedulerResources>>) {
        // Sleep until new process is added to the ready queue.
        let all_procs_are_finished_condvar = {
            let resources = resources.lock().unwrap();
            resources.all_procs_are_finished_condvar.clone()
        };

        {
            let (lock, condvar) = &*all_procs_are_finished_condvar;
            let mut all_procs_are_finished = lock.lock().unwrap();

            while *all_procs_are_finished {
                all_procs_are_finished = condvar.wait(all_procs_are_finished).unwrap();
            }
        }

        // Dispatch process.
        let (cpu, in_pcb, out_pcb) = {
            let mut resources = resources.lock().unwrap();

            let cpu = resources.cpu.clone();
            let in_pcb;
            let out_pcb = resources.current_pcb.clone();

            if resources.ready_queue.is_empty() {
                in_pcb = None;
                resources.current_pcb = None;
            } else {
                in_pcb = resources.ready_queue.pop();
                resources.current_pcb = in_pcb.clone();
            }

            (cpu, in_pcb, out_pcb)
        };
        
        let in_pcb_clone = in_pcb.clone();
        let out_pcb_clone = out_pcb.clone();
        let out_pcb_state;

        let mut cpu = cpu.lock().unwrap();

        // Block until current process is done.
        out_pcb_state = cpu.await_process_interrupt();

        // Puts the process on the CPU.
        // The process currently on the CPU is moved out of the CPU and its PCB is updated.
        cpu.execute_process(in_pcb, out_pcb);

        if out_pcb_clone.is_none() {
            return;
        }

        // Handle the process based on its state after being moved out of the CPU.
        let mut resources = resources.lock().unwrap();
        match out_pcb_state {
            ProcessState::Ready => { // This would be the case if the process was not finished executing.
                out_pcb_clone.as_ref().unwrap().lock().unwrap().state = ProcessState::Ready;
                resources.ready_queue.push(out_pcb_clone.unwrap());
            },
            ProcessState::Waiting => { // This is unimplemented due to lack of I/O devices.
                out_pcb_clone.as_ref().unwrap().lock().unwrap().state = ProcessState::Waiting;
                // Put process in waiting queue.
            },
            ProcessState::Terminated => { 
                let out_pcb = out_pcb_clone.unwrap();
                let mut out_pcb = out_pcb.lock().unwrap();

                out_pcb.state = ProcessState::Terminated;
                out_pcb.end_record_turnaround_time(); // Finish recording turnaround time.
            },
            ProcessState::Running => {
                panic!("Process should not be set to running after being moved out of the CPU.");
            },
        }

        // Notify main thread if there are no more processes to execute.
        if in_pcb_clone.is_none() {
            let (lock, condvar) = &*resources.all_procs_are_finished_condvar;
            let mut all_procs_are_finished = lock.lock().unwrap();

            *all_procs_are_finished = true;
            condvar.notify_all();
        }
    }
}

impl Drop for ShortTermScheduler {
    /// Signals the dispatch thread to terminate when the short-term scheduler is dropped.
    fn drop(&mut self) {
        self.dispatch_should_terminate.store(true, Ordering::Relaxed);
    }
}

struct ShortTermSchedulerResources {
    cpu: Arc<Mutex<Cpu>>,
    ready_queue: Box<dyn SchedulerQueue + Send>,
    current_pcb: Option<Arc<Mutex<ProcessControlBlock>>>,
    all_procs_are_finished_condvar: Arc<(Mutex<bool>, Condvar)>,
}

impl ShortTermSchedulerResources {
    pub fn new(cpu: Arc<Mutex<Cpu>>, ready_queue: Box<dyn SchedulerQueue + Send>) -> ShortTermSchedulerResources {
        ShortTermSchedulerResources {
            cpu,
            ready_queue,
            current_pcb: None,
            all_procs_are_finished_condvar: Arc::new((Mutex::new(true), Condvar::new())),
        }
    }
}

trait SchedulerQueue {
    fn push(&mut self, pcb: Arc<Mutex<ProcessControlBlock>>);
    fn pop(&mut self) -> Option<Arc<Mutex<ProcessControlBlock>>>;
    fn is_empty(&self) -> bool;
}

struct FifoQueue {
    queue: VecDeque<Arc<Mutex<ProcessControlBlock>>>,
}

impl FifoQueue {
    pub fn new() -> FifoQueue {
        FifoQueue {
            queue: VecDeque::new(),
        }
    }
}

impl SchedulerQueue for FifoQueue {
    fn push(&mut self, pcb: Arc<Mutex<ProcessControlBlock>>) {
        self.queue.push_back(pcb);
    }

    fn pop(&mut self) -> Option<Arc<Mutex<ProcessControlBlock>>> {
        self.queue.pop_front()
    }

    fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}

struct PriorityQueue {
    queue: BinaryHeap<PriorityProcessControlBlock>,
}

impl PriorityQueue {
    pub fn new() -> PriorityQueue {
        PriorityQueue {
            queue: BinaryHeap::new(),
        }
    }
}

impl SchedulerQueue for PriorityQueue {
    fn push(&mut self, pcb: Arc<Mutex<ProcessControlBlock>>) {
        let priority_pcb = PriorityProcessControlBlock::new(pcb);
        self.queue.push(priority_pcb);
    }

    fn pop(&mut self) -> Option<Arc<Mutex<ProcessControlBlock>>> {
        let priority_pcb = self.queue.pop()?;
        Some(priority_pcb.pcb)
    }

    fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}

struct PriorityProcessControlBlock {
    pcb: Arc<Mutex<ProcessControlBlock>>,
}

impl PriorityProcessControlBlock {
    pub fn new(pcb: Arc<Mutex<ProcessControlBlock>>) -> PriorityProcessControlBlock {
        PriorityProcessControlBlock {
            pcb,
        }
    }
}

impl PartialEq for PriorityProcessControlBlock {
    fn eq(&self, other: &Self) -> bool {
        self.pcb.lock().unwrap().get_priority() == other.pcb.lock().unwrap().get_priority()
    }
}

impl Eq for PriorityProcessControlBlock {}

impl PartialOrd for PriorityProcessControlBlock {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PriorityProcessControlBlock {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.pcb.lock().unwrap().get_priority().cmp(&other.pcb.lock().unwrap().get_priority())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::io::ProgramInfo;

    #[test]
    fn test_priority_queue() {
        let program_info_1 = ProgramInfo {
            id: 0,
            priority: 1,
            instruction_buffer_size: 0,
            in_buffer_size: 0,
            out_buffer_size: 0,
            temp_buffer_size: 0,
            data_start_idx: 0,
        };
        let program_info_2 = ProgramInfo {
            id: 1,
            priority: 2,
            instruction_buffer_size: 0,
            in_buffer_size: 0,
            out_buffer_size: 0,
            temp_buffer_size: 0,
            data_start_idx: 0,
        };
        let program_info_3 = ProgramInfo {
            id: 2,
            priority: 3,
            instruction_buffer_size: 0,
            in_buffer_size: 0,
            out_buffer_size: 0,
            temp_buffer_size: 0,
            data_start_idx: 0,
        };

        let pcb_1 = Arc::new(Mutex::new(ProcessControlBlock::new(&program_info_1, 0, 0)));
        let pcb_2 = Arc::new(Mutex::new(ProcessControlBlock::new(&program_info_2, 0, 0)));
        let pcb_3 = Arc::new(Mutex::new(ProcessControlBlock::new(&program_info_3, 0, 0)));

        let mut queue = PriorityQueue::new();
        queue.push(pcb_2.clone());
        queue.push(pcb_1.clone());
        queue.push(pcb_3.clone());

        assert_eq!(queue.pop().unwrap().lock().unwrap().get_id(), 2);
        assert_eq!(queue.pop().unwrap().lock().unwrap().get_id(), 1);
        assert_eq!(queue.pop().unwrap().lock().unwrap().get_id(), 0);
    }
}