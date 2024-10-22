use core::panic;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::io::ProgramInfo;

#[derive(Clone, Copy)]
pub(crate) enum ProcessState {
    Ready,
    Running,
    Waiting,
    Terminated,
}

pub(crate) struct ProcessControlBlock {
    pub program_counter: usize,
    pub registers: [u32; 16],
    pub state: ProcessState,
    
    id: u32,
    priority: u32,

    mem_start_address: usize,
    mem_in_start_address: usize,
    mem_out_start_address: usize,
    mem_temp_start_address: usize,
    mem_end_address: usize,

    turnaround_time_ms: f64,
    turnaround_start_time_ns: u128,
    turnaround_time_is_recording: bool,

    burst_times_ms: Vec<f64>,
    current_burst_start_time_ns: u128,
    burst_time_is_recording: bool,
}

impl ProcessControlBlock {
    pub fn new(program_info: &ProgramInfo, mem_start_address: usize, mem_end_address: usize) -> ProcessControlBlock {
        ProcessControlBlock {
            program_counter: 0,
            registers: [0; 16],
            state: ProcessState::Ready,

            id: program_info.id,
            priority: program_info.priority,

            mem_start_address,
            mem_in_start_address: mem_start_address + program_info.instruction_buffer_size,
            mem_out_start_address: mem_start_address + program_info.instruction_buffer_size + program_info.in_buffer_size,
            mem_temp_start_address: mem_start_address + program_info.instruction_buffer_size + program_info.in_buffer_size + program_info.out_buffer_size,
            mem_end_address,

            turnaround_time_ms: 0.0,
            turnaround_start_time_ns: 0,
            turnaround_time_is_recording: false,

            burst_times_ms: Vec::new(),
            current_burst_start_time_ns: 0,
            burst_time_is_recording: false,
        }
    }

    pub fn get_id(&self) -> u32 {
        self.id
    }

    pub fn get_priority(&self) -> u32 {
        self.priority
    }

    pub fn get_mem_start_address(&self) -> usize {
        self.mem_start_address
    }

    pub fn get_mem_in_start_address(&self) -> usize {
        self.mem_in_start_address
    }

    pub fn get_mem_out_start_address(&self) -> usize {
        self.mem_out_start_address
    }

    pub fn get_mem_temp_start_address(&self) -> usize {
        self.mem_temp_start_address
    }

    pub fn get_mem_end_address(&self) -> usize {
        self.mem_end_address
    }

    pub fn start_record_turnaround_time(&mut self) {
        self.turnaround_start_time_ns = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        self.turnaround_time_is_recording = true;
    }

    pub fn end_record_turnaround_time(&mut self) {
        if self.turnaround_time_is_recording == false {
            panic!("Process time is not being recorded.");
        }

        let turnaround_end_time_ms = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let turnaround_time_ns = turnaround_end_time_ms - self.turnaround_start_time_ns;
        let turnaround_time_ms = turnaround_time_ns as f64 / 1_000_000.0;

        self.turnaround_time_ms = turnaround_time_ms;
        self.turnaround_time_is_recording = false;
    }

    pub fn get_turnaround_time_ms(&self) -> f64 {
        self.turnaround_time_ms
    }

    pub fn start_record_burst_time(&mut self) {
        self.current_burst_start_time_ns = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        self.burst_time_is_recording = true;
    }

    pub fn end_record_burst_time(&mut self) {
        if self.burst_time_is_recording == false {
            panic!("Burst time is not being recorded.");
        }

        let current_burst_end_time_ns = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let current_burst_time_ns = current_burst_end_time_ns - self.current_burst_start_time_ns;
        let current_burst_time_ms = current_burst_time_ns as f64 / 1_000_000.0;

        self.burst_times_ms.push(current_burst_time_ms);
        self.burst_time_is_recording = false;
    }

    pub fn get_avg_burst_time_ms(&self) -> f64 {
        if self.burst_times_ms.is_empty() {
            return 0.0;
        }

        let total_burst_time_ms: f64 = self.burst_times_ms.iter().sum();
        total_burst_time_ms / self.burst_times_ms.len() as f64
    }
}