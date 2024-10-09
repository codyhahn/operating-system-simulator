use std::fs::File;
use std::io::prelude::*;

fn coredump(memory : Vecdeque<u32>) -> std::io::Result<()> {
    let mut file = File::create("core dump");
    while memory != 0 {
    file.write(memory.pop_front());
    }
}