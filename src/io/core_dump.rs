use std::fs::File;
use std::io::prelude::*;

fn coredump(memory : Vecdeque<u32>) -> std::io::Result<()> {
    let mut file = File::create("core dump");

    //write the jobs
    file.write("JOB")
    while memory != 0 & memory.len() > 15 {
    file.write(memory.pop_front());
    }
    
    //write the data
    file.write("DATA")
    while memory != 0 {
        file.write(memory.pop_front());
    }
}