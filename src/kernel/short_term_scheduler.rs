use std::collections::{BinaryHeap, VecDeque};
use std::sync::{Arc, Condvar, Mutex, atomic::{AtomicBool, Ordering}};
use std::thread;

use super::{Cpu, ProcessControlBlock, ProcessState};

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
        //let should_notify = resources.ready_queue.is_empty();
        
        resources.ready_queue.push(pcb);

        //if should_notify {
        //    let (lock, condvar) = &resources.ready_queue_condvar;
        //    let _ = lock.lock().unwrap();
        //    condvar.notify_one();
        //}
    }

    fn dispatch(resources: &Arc<Mutex<ShortTermSchedulerResources>>) {
        //{
        //    let (lock, condvar) = &resources.ready_queue_condvar;
        //    let ready_queue_lock = lock.lock().unwrap();
        //

        let ready_queue = {
            &resources.lock().unwrap().ready_queue
        };

        while ready_queue.is_empty() {}

        let mut resources = resources.lock().unwrap();

        println!("Dispatching process.");

        let in_pcb = resources.ready_queue.pop().unwrap();
        let in_pcb_clone = in_pcb.clone();

        let out_pcb = resources.current_pcb.clone();
        let out_pcb_clone = out_pcb.clone();
        let out_pcb_state;

        {
            let mut cpu = resources.cpu.lock().unwrap();
            out_pcb_state = cpu.await_process_interrupt(); // Blocks until process is either done or waiting.
            cpu.execute_process(in_pcb, out_pcb);
        }

        resources.current_pcb = Some(in_pcb_clone);
        {
            resources.current_pcb.as_ref().unwrap().lock().unwrap().state = ProcessState::Running;
        }

        match out_pcb_state {
            ProcessState::Ready => {
                out_pcb_clone.as_ref().unwrap().lock().unwrap().state = ProcessState::Ready;
                resources.ready_queue.push(out_pcb_clone.unwrap());
            },
            ProcessState::Waiting => {
                out_pcb_clone.as_ref().unwrap().lock().unwrap().state = ProcessState::Waiting;
                // TODO: Implement waiting queue.
            },
            ProcessState::Terminated => {},
            ProcessState::Running => {
                panic!("Process should not set to running after being moved out of the CPU.");
            },
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
    ready_queue_condvar: (Mutex<()>, Condvar),
    current_pcb: Option<Arc<Mutex<ProcessControlBlock>>>,
}

impl ShortTermSchedulerResources {
    pub fn new(cpu: Arc<Mutex<Cpu>>, ready_queue: Box<dyn SchedulerQueue + Send>) -> ShortTermSchedulerResources {
        ShortTermSchedulerResources {
            cpu,
            ready_queue,
            ready_queue_condvar: (Mutex::new(()), Condvar::new()),
            current_pcb: None,
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