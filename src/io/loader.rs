use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

use super::Disk;

const PROGRAM_FILE_PATH: &str = "data/program_file.txt";
const OUT_PATH: &str = "out";

pub fn load_programs_into_disk(disk: &mut Disk) -> std::io::Result<Vec<u32>> {
    let file = File::open(PROGRAM_FILE_PATH)?;
    let reader = BufReader::new(file);

    let mut program_ids = Vec::new();

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

            program_ids.push(id);
            data.clear();
        } else {
            let line = line.trim();
            let value = u32::from_str_radix(&line[2..], 16).unwrap_or_else(|_| {
                panic!("Failed to parse hex value: {}", line)
            });

            data.push(value);
        }
    }
    
    Ok(program_ids)
}

pub fn write_disk_to_file(disk: &Disk) {
    if !Path::new(OUT_PATH).exists() {
        fs::create_dir(OUT_PATH).unwrap();
    }

    let filename = format!("{}/program_file_executed.txt", OUT_PATH);
    let mut file = File::create(filename).unwrap();

    let program_infos = disk.get_program_infos(true);

    for program_info in program_infos {
        let data = disk.read_data_for(&program_info);

        writeln!(file, "// JOB {:X} {:X} {:X}", program_info.id, program_info.instruction_buffer_size, program_info.priority).unwrap();

        for i in 0..program_info.instruction_buffer_size {
            writeln!(file, "0x{:08X}", data[i]).unwrap();
        }

        writeln!(file, "// Data {:X} {:X} {:X}", program_info.in_buffer_size, program_info.out_buffer_size, program_info.temp_buffer_size).unwrap();

        let start_idx = program_info.instruction_buffer_size;
        let end_idx = start_idx
                             + program_info.in_buffer_size
                             + program_info.out_buffer_size
                             + program_info.temp_buffer_size;
        for i in start_idx..end_idx {
            writeln!(file, "0x{:08X}", data[i]).unwrap();
        }

        writeln!(file, "// END").unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_programs_into_disk() {
        let mut disk = Disk::new();
        load_programs_into_disk(&mut disk).unwrap();

        let program = disk.get_info_for(1);

        assert_eq!(program.id, 1);
        assert_eq!(program.priority, 2);
        assert_eq!(program.instruction_buffer_size, 23);
        assert_eq!(program.in_buffer_size, 20);
        assert_eq!(program.out_buffer_size, 12);
        assert_eq!(program.temp_buffer_size, 12);

        let program = disk.get_info_for(30);

        assert_eq!(program.id, 30);
        assert_eq!(program.priority, 8);
        assert_eq!(program.instruction_buffer_size, 19);
        assert_eq!(program.in_buffer_size, 20);
        assert_eq!(program.out_buffer_size, 12);
        assert_eq!(program.temp_buffer_size, 12);
    }
}