use std::collections::{BinaryHeap, VecDeque};

use super::{Memory, ProcessControlBlock};

pub(crate) enum SchedulingAlgorithm {
    Fifo,
    Priority,
}

pub(crate) struct ShortTermScheduler {
    fifo_queue: VecDeque<ProcessControlBlock>,
    priority_queue: BinaryHeap<ProcessControlBlock>,
    scheduling_alg: SchedulingAlgorithm,
}

impl ShortTermScheduler {
    pub fn new(scheduling_alg: SchedulingAlgorithm) -> ShortTermScheduler {
        ShortTermScheduler {
            priority_queue: BinaryHeap::new(),
            fifo_queue: VecDeque::new(),
            scheduling_alg,
        }
    }

    pub fn schedule_process(&mut self, pcb: ProcessControlBlock) {
        match self.scheduling_alg {
            SchedulingAlgorithm::Fifo => self.fifo_queue.push_back(pcb),
            SchedulingAlgorithm::Priority => self.priority_queue.push(pcb),
        }
    }

    pub fn schedule_next(&mut self, memory: &Memory) -> Option<ProcessControlBlock> {
        match self.scheduling_algo {
            SchedulingAlgorithm::Fifo => self.fifo_queue.pop_front(),
            SchedulingAlgorithm::Priority => self.priority_queue.pop(),
        }
    }   
}
