use std::collections::{BinaryHeap, VecDeque};
use std::sync::{Arc, Condvar, Mutex, atomic::{AtomicBool, Ordering}};
use std::thread;

use super::ProcessControlBlock;

pub(crate) trait SchedulerQueue {
    fn push(&mut self, pcb: Arc<Mutex<ProcessControlBlock>>);
    fn pop(&mut self) -> Option<Arc<Mutex<ProcessControlBlock>>>;
    fn is_empty(&self) -> bool;
}

pub(crate) struct FifoQueue {
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

pub(crate) struct PriorityQueue {
    queue: BinaryHeap<Arc<Mutex<ProcessControlBlock>>>,
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
        self.queue.push(pcb);
    }

    fn pop(&mut self) -> Option<Arc<Mutex<ProcessControlBlock>>> {
        self.queue.pop()
    }

    fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}

pub(crate) struct ShortTermScheduler {
    ready_queue: Arc<Mutex<Box<dyn SchedulerQueue + Send>>>,
    ready_queue_condvar: Arc<Condvar>,
    dispatch_kill_flag: Arc<AtomicBool>,
}

impl ShortTermScheduler {
    pub fn new(scheduler_queue: Box<dyn SchedulerQueue + Send>) -> ShortTermScheduler {
        let ready_queue = Arc::new(Mutex::new(scheduler_queue));
        let ready_queue_condvar = Arc::new(Condvar::new());
        let dispatch_kill_flag = Arc::new(AtomicBool::new(false));

        let ready_queue_clone = ready_queue.clone();
        let ready_queue_condvar_clone = ready_queue_condvar.clone();
        let dispatch_kill_flag_clone = dispatch_kill_flag.clone();

        thread::spawn(move || {
            while !dispatch_kill_flag_clone.load(Ordering::Relaxed) {
                ShortTermScheduler::dispatch(&ready_queue_clone, &ready_queue_condvar_clone);
            }
        });

        ShortTermScheduler {
            ready_queue,
            ready_queue_condvar,
            dispatch_kill_flag,
        }
    }

    pub fn schedule_process(&mut self, pcb: Arc<Mutex<ProcessControlBlock>>) {
        let queue_lock = &self.ready_queue;
        let condvar = &self.ready_queue_condvar;
        
        let mut ready_queue = queue_lock.lock().unwrap();

        ready_queue.push(pcb);
        condvar.notify_one();
    }

    fn dispatch(ready_queue: &Arc<Mutex<Box<dyn SchedulerQueue + Send>>>, ready_queue_condvar: &Arc<Condvar>) {
        let mut ready_queue = ready_queue.lock().unwrap();

        while ready_queue.is_empty() {
            ready_queue = ready_queue_condvar.wait(ready_queue).unwrap();
        }

        let pcb = ready_queue.pop();

        // Send pcb to cpu.
    }
}

impl Drop for ShortTermScheduler {
    fn drop(&mut self) {
        self.dispatch_kill_flag.store(true, Ordering::Relaxed);
    }
}