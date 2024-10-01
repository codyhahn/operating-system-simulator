use ::std::fs;

use crate::driver::Status;
use crate::driver::PCB;

/// Loads the data from the program file into the virtual system.
/// This includes adding program instructions/data to disk,
/// as well as filling the PCBs.
pub struct Loader {}

impl Loader {
    /// Reads the program file. Fills the disk with program data
    /// and creates PCB entries.
    pub fn load() {
        let program_file: String =
            fs::read_to_string("../program_file").expect("Should be able to read the file");

        for line in program_file.lines() {
            if line.starts_with("// JOB") {
                let pcb = Self::parse_job(&line[7..]);
            }
        }
    }

    /// Parses the JOB control card line of a program in the PF.
    fn parse_job(control_card: &str) -> PCB {
        let inputs = &mut control_card.split_ascii_whitespace();

        let mut job_metadata: [u32; 3] = [0; 3];

        for i in 0..3 {
            job_metadata[i] = u32::from_str_radix(
                inputs.next().unwrap_or_else(|| {
                    panic!("JOB control card is missing value");
                }),
                16,
            )
            .unwrap_or_else(|err| {
                panic!("JOB control card is not a hex value: {err}");
            });
        }

        PCB {
            id: job_metadata[0],
            instruction_size: job_metadata[1],
            priority: job_metadata[2],
            status: Status::New,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_job_line() {
        let job_line = "// JOB 1 17 2";
        let pcb = Loader::parse_job(&job_line[7..]);
        assert_eq!(1, pcb.id);
        assert_eq!(23, pcb.instruction_size);
        assert_eq!(2, pcb.priority);
        assert_eq!(Status::New, pcb.status);
    }

    #[test]
    #[should_panic(expected = "missing value")]
    fn missing_value() {
        let job_line = "// JOB 4 13";
        Loader::parse_job(&job_line[7..]);
    }

    #[test]
    #[should_panic(expected = "not a hex value")]
    fn invalid_value() {
        let job_line = "// JOB NOT HEX DATA";
        Loader::parse_job(&job_line[7..]);
    }
}
