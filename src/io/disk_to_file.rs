use std::fs::File;
use std::io::prelude::*;
use crate::io::Disk;

pub fn diskdata_to_file(disk : Disk) -> std::io::Result<()> {

    let mut file = File::create("core dump data")?;
    
    //write the data on the disk to file
    for job in disk.data.iter(){
        writeln!(file,"{}",job)?;
        
    }

    Ok(())
}

pub fn diskpcb_to_file(disk : Disk){

  let mut file = File::create("core dump pcb");

  //write program info into file
  for job in disk.program_map.iter(){



  }
  

  
}


    