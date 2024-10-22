use std::fs::File;
use std::io::prelude::*;
use crate::io::Disk;

struct disk_to_file{
}

impl disk_to_file for Disk{
  fn disk_to_file(data : &Disk) -> std::io::Result<()> {

    let mut file = File::create("core dump")?;
    
    
    let mut data = data;
    //write the data on the disk to file
    for job in data{
        file.write(job);
        
    };


    

    Ok(())
}
}

    