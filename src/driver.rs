use crate::loader::Loader;

/// Holds the virtual system environment,
/// and calls the other subsystems threrein.
pub struct Driver {
    pcbs: Vec<PCB>,
}

impl Driver {
    pub fn start() {
        Loader::load();
    }
}

/// The process control block. Holds process metadata.
pub struct PCB {
    pub id: u32,
    pub instruction_size: u32,
    pub priority: u32,
    pub status: Status,
}

/// Process queue status.
/// Provides information on the current state of the process.
#[derive(Debug, PartialEq)]
pub enum Status {
    New,
    Running,
}
