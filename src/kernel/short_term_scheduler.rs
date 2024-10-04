/// Loads processes from RAM to the CPU.
/// sets the CPU environment to the state described in the PCB.
use std::collections::{BinaryHeap, VecDeque};

use super::{Memory, ProcessControlBlock};

// determines current algorithm
pub enum SchedulingAlgorithm {
    Priority,
    FIFO,
}

pub struct ShortTermScheduler {
    fifo_queue: VecDeque<ProcessControlBlock>,
    priority_queue: BinaryHeap<ProcessControlBlock>,
    scheduling_algo: SchedulingAlgorithm,
}

impl ShortTermScheduler {
    pub fn new(scheduling_algo: SchedulingAlgorithm) -> ShortTermScheduler {
        ShortTermScheduler {
            priority_queue: BinaryHeap::new(),
            fifo_queue: VecDeque::new(),
            scheduling_algo,
        }
    }

    // adds process to the correct queue based on algorithm
    pub fn add_process(&mut self, pcb: ProcessControlBlock) {
        match self.scheduling_algo {
            SchedulingAlgorithm::Priority => self.priority_queue.push(pcb),
            SchedulingAlgorithm::FIFO => self.fifo_queue.push_back(pcb),
        }
    }

    // schedule process based on algorithm
    pub fn schedule_next(&mut self, memory: &Memory) -> Option<ProcessControlBlock> {
        match self.scheduling_algo {
            SchedulingAlgorithm::Priority => self.priority_queue.pop(),
            SchedulingAlgorithm::FIFO => self.fifo_queue.pop_front(),
        }
    }   
}
