use super::Memory;

pub struct Driver {
    memory: Memory,
}

impl Driver {
    pub fn new() -> Driver {
        Driver {
            memory: Memory::new()
        }
    }

    pub fn start(&mut self) {}
}