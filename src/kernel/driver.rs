use super::Memory;

pub struct Driver {
    disk: Disk,
    memory: Memory,
}

impl Driver {
    pub fn new() -> Driver {
        Driver {
            disk: Disk::new(),
            memory: Memory::new()
        }
    }

    pub fn start(&mut self) {}
}