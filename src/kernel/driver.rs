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

    pub fn start(&mut self) {
        if loader::load_programs_into_disk(&mut self.disk).is_ok() {
            println!("Programs loaded into disk successfully");
        } else {
            println!("Failed to load programs into disk");
        }
    }
}