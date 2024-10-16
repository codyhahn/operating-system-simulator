use std::collections::{BinaryHeap, VecDeque};
use std::sync::{Arc, Condvar, Mutex, atomic::{AtomicBool, Ordering}};
use std::thread;

use super::{Cpu, ProcessControlBlock, ProcessState};

#[allow(dead_code)]
pub(crate) enum StsSchedulingAlg {
    Fifo,
    Priority,
}

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

    pub fn schedule_process(&mut self, pcb: Arc<Mutex<ProcessControlBlock>>) {
        let mut resources = self.resources.lock().unwrap();
        resources.ready_queue.push(pcb);

        let (lock, condvar) = &*resources.all_procs_are_finished_condvar;
        let mut all_procs_are_finished = lock.lock().unwrap();

        *all_procs_are_finished = false;
        condvar.notify_all();
    }

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
            let in_pcb = resources.ready_queue.pop().unwrap();
            let out_pcb = resources.current_pcb.clone();
            resources.current_pcb = Some(in_pcb.clone());

            (cpu, in_pcb, out_pcb)
        };
        
        let out_pcb_clone = out_pcb.clone();
        let out_pcb_state;

        let mut cpu = cpu.lock().unwrap();
        out_pcb_state = cpu.await_process_interrupt(); // Blocks until current process is done.

        in_pcb.lock().unwrap().state = ProcessState::Running;
        cpu.execute_process(in_pcb, out_pcb);

        let mut resources = resources.lock().unwrap();
        match out_pcb_state {
            ProcessState::Ready => {
                out_pcb_clone.as_ref().unwrap().lock().unwrap().state = ProcessState::Ready;
                resources.ready_queue.push(out_pcb_clone.unwrap());
            },
            ProcessState::Waiting => {
                out_pcb_clone.as_ref().unwrap().lock().unwrap().state = ProcessState::Waiting;
                // Unimplemented due to lack of I/O devices and therefore DMA channel.
            },
            ProcessState::Terminated => { /* Do nothing. */ },
            ProcessState::Running => {
                panic!("Process should not be set to running after being moved out of the CPU.");
            },
        }

        // Notify all processes are finished if ready queue is empty.
        if resources.ready_queue.is_empty() {
            let (lock, condvar) = &*resources.all_procs_are_finished_condvar;
            let mut all_procs_are_finished = lock.lock().unwrap();

            *all_procs_are_finished = true;
            condvar.notify_all();
        }
    }
}

impl Drop for ShortTermScheduler {
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