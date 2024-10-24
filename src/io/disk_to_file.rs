use std::fs::File;
use std::io::prelude::*;
use crate::io::Disk;

pub fn diskdata_to_file(disk : &Disk) -> std::io::Result<()> {

    let mut file_data = File::create("core dump data")?;
    
    //write the data on the disk to file
    for job in disk.data.iter(){
        writeln!(file_data,"{}",job)?;
        
    }

    Ok(())
}

/*pub fn diskpcb_to_file(disk : &Disk)-> std::io::Result<()>{

  let mut file_pcb = File::create("core dump pcb")?;

  //write program info into file
  for job in disk.program_map.iter(){

  write!(file_pcb,"id: ")?;
  writeln!(file_pcb,"{}", disk<id,job>)?;
  writeln! 

  }

  Ok(())
  

  
} */

mod tests{
    use crate::io::Disk;
    use crate::io::disk_to_file;

  #[test]
  fn test(){
  let mut disk = Disk::new();
  disk.write_program(2,2,2,2,2,3, &[5,6,7,8,9]);
  disk_to_file.diskdata_to_file(disk);
  }
  
}




    