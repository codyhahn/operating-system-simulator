use std::fs::File;
use std::io::{BufRead, BufReader};

use super::Disk;

const PROGRAM_FILE_PATH: &str = "data/program_file.txt";

pub fn load_programs_into_disk(disk: &mut Disk) -> std::io::Result<()> {
    let file = File::open(PROGRAM_FILE_PATH)?;
    let reader = BufReader::new(file);

    let mut data = Vec::new();
    let mut id = 0;
    let mut priority = 0;
    let mut instruction_buffer_size = 0;
    let mut in_buffer_size = 0;
    let mut out_buffer_size = 0;
    let mut temp_buffer_size = 0;

    for line in reader.lines() {
        let line = line?;

        if line.starts_with("// JOB") {
            let job_info = &line[7..];
            let job_info: Vec<&str> = job_info.split_whitespace().collect();

            id = u32::from_str_radix(job_info[0], 16).unwrap();
            instruction_buffer_size = usize::from_str_radix(job_info[1], 16).unwrap();
            priority = u32::from_str_radix(job_info[2], 16).unwrap();
        } else if line.starts_with("// Data") {
            let data_info = &line[8..];
            let data_info: Vec<&str> = data_info.split_whitespace().collect();

            in_buffer_size = usize::from_str_radix(data_info[0], 16).unwrap();
            out_buffer_size = usize::from_str_radix(data_info[1], 16).unwrap();
            temp_buffer_size = usize::from_str_radix(data_info[2], 16).unwrap();
        } else if line.starts_with("// END") {
            disk.write_program(id,
                               priority,
                               instruction_buffer_size,
                               in_buffer_size,
                               out_buffer_size,
                               temp_buffer_size,
                               data.as_slice());

            data.clear();
        } else {
            let line = line.trim();
            let value = u32::from_str_radix(&line[2..], 16).unwrap_or_else(|_| {
                panic!("Failed to parse hex value: {}", line)
            });

            data.push(value);
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_programs_into_disk() {
        let mut disk = Disk::new();
        load_programs_into_disk(&mut disk).unwrap();

        let program = disk.read_program(1);

        assert_eq!(program.id, 1);
        assert_eq!(program.priority, 2);
        assert_eq!(program.instruction_buffer_size, 23);
        assert_eq!(program.in_buffer_size, 20);
        assert_eq!(program.out_buffer_size, 12);
        assert_eq!(program.temp_buffer_size, 12);

        let program = disk.read_program(30);

        assert_eq!(program.id, 30);
        assert_eq!(program.priority, 8);
        assert_eq!(program.instruction_buffer_size, 19);
        assert_eq!(program.in_buffer_size, 20);
        assert_eq!(program.out_buffer_size, 12);
        assert_eq!(program.temp_buffer_size, 12);
    }
}