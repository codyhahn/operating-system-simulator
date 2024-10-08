use std::cell::RefCell;
use std::sync::{Arc, Mutex};

use super::{ProcessControlBlock, Memory};

pub struct Cpu {
    memory:Arc<RefCell<Memory>>,
    cache:Vec<u32>,
    pcb:Option<Arc<Mutex<ProcessControlBlock>>>,
    is_running:bool,
}

impl Cpu {
    pub fn new(memory: Arc<RefCell<Memory>>) -> Cpu {
        Cpu {
            memory,
            cache:Vec::new(),
            pcb:None,
            is_running:false,
        }
    }

    pub fn execute_process(&mut self, pcb: Arc<Mutex<ProcessControlBlock>>) {
        self.pcb = Some(pcb);

        let pcb = self.pcb.as_ref().unwrap().lock().unwrap();
        self.cache = self.memory.borrow().read_block_from(pcb.mem_in_start_address, pcb.mem_out_start_address);
    }

    fn cycle(&mut self) {
        let mut pcb = self.pcb.as_ref().unwrap().lock().unwrap();

        let current_instruction = self.cache[pcb.program_counter];
        pcb.program_counter += 1;
        drop(pcb);

        let decoded_instruction = self.decode(current_instruction);

        self.execute(decoded_instruction);
    }

    fn decode(&self, instruction: u32) -> DecodedInstruction {
        let mut result = DecodedInstruction::new();
        
        // Get instruction type (bits 0-1).
        result.instr_type = match self.extract_bits(instruction, 0, 2){
            0b00 => InstructionType::Arithmetic,
            0b01 => InstructionType::CondBranchImmediate,
            0b10 => InstructionType::UncondJump,
            0b11 => InstructionType::IO,
            _ => panic!("Execute error, invalid instruction type"),
        };

        // Get opcode (bits 2-6).
        result.opcode = self.extract_bits(instruction, 2, 6).try_into().unwrap();

        // Get register values and address based on instruction type.
        match result.instr_type{
            InstructionType::Arithmetic => {
                result.reg1 = self.extract_bits(instruction, 8, 4).try_into().unwrap();
                result.reg2 = self.extract_bits(instruction, 12, 4).try_into().unwrap();
                result.reg3 = self.extract_bits(instruction, 16, 4).try_into().unwrap();
            },
            InstructionType::CondBranchImmediate => {
                result.reg1 = self.extract_bits(instruction, 8, 4).try_into().unwrap();
                result.reg2 = self.extract_bits(instruction, 12, 4).try_into().unwrap();
                result.address = self.extract_bits(instruction, 16,16).try_into().unwrap();
            },
            InstructionType::UncondJump => {
                result.address = self.extract_bits(instruction, 8, 16).try_into().unwrap();
            },
            InstructionType::IO => {
                result.reg1 = self.extract_bits(instruction, 8, 4).try_into().unwrap();
                result.reg2 = self.extract_bits(instruction, 12, 4).try_into().unwrap();
                result.address = self.extract_bits(instruction, 16, 16).try_into().unwrap();
            },
        }

        result
    }

    fn extract_bits(&self, instruction: u32, start_index: u32, length: u32) -> u32 {
        (instruction << start_index) >> (32 - length)
    }

    fn execute(&mut self, instruction: DecodedInstruction) {
        // No-op.
        if instruction.opcode == 0x13 {
            return;
        }

        match instruction.instr_type {
            InstructionType::Arithmetic => self.execute_arithmetic(instruction),
            InstructionType::CondBranchImmediate => self.execute_cond_branch_immediate(instruction),
            InstructionType::UncondJump => self.execute_uncond_jump(instruction),
            InstructionType::IO => self.execute_io(instruction),
        }
    }

    fn execute_arithmetic(&mut self, instruction: DecodedInstruction) {
        match instruction.opcode {
            0x4 => /*MOV*/ self.set_reg(instruction.reg2, self.get_reg(instruction.reg1)),
            0x5 => /*ADD*/ self.set_reg(instruction.reg1, self.get_reg(instruction.reg2) + self.get_reg(instruction.reg3)),
            0x6 => /*SUB*/ self.set_reg(instruction.reg1, self.get_reg(instruction.reg2) - self.get_reg(instruction.reg3)),
            0x7 => /*MUL*/ self.set_reg(instruction.reg1, self.get_reg(instruction.reg2) * self.get_reg(instruction.reg3)),
            0x8 => /*DIV*/ self.set_reg(instruction.reg1, self.get_reg(instruction.reg2) / self.get_reg(instruction.reg3)),
            0x9 => /*AND*/ self.set_reg(instruction.reg1, self.get_reg(instruction.reg2) & self.get_reg(instruction.reg3)),
            0xA => /*OR */ self.set_reg(instruction.reg1, self.get_reg(instruction.reg2) | self.get_reg(instruction.reg3)),
            0x10 => /*SLT*/ {
                if self.get_reg(instruction.reg1) < self.get_reg(instruction.reg2){
                    self.set_reg(instruction.reg3, 1);
                }
                else{
                    self.set_reg(instruction.reg3, 0);
                }
            },
            _ => panic!("Execute error, invalid opcode for arithmetic instruction"),
        };
    }

    fn execute_cond_branch_immediate(&mut self, instruction: DecodedInstruction) {
        match instruction.opcode{
            0x2 =>  /* ST */ {
                if self.get_reg(instruction.reg2) == 0 {
                    //self.memory.write_to(instruction.address as usize, self.get_reg(instruction.reg1));
                }
                else {
                    //self.memory.write_to(self.get_reg(instruction.reg2) as usize, self.get_reg(instruction.reg1));
                }
            },
            0x3 =>  /* LW */ {
                if self.get_reg(instruction.reg2) == 0 {
                    //self.set_reg(instruction.reg1, self.fetch(instruction.address));
                }
                else {
                    //self.set_reg(instruction.reg1, self.fetch(self.get_reg(instruction.reg2) as usize));
                }
            },
            0xB =>  /* MOVI */ self.set_reg(instruction.reg2, instruction.address as u32),
            0xC =>  /* ADDI */ self.set_reg(instruction.reg2, self.get_reg(instruction.reg2) + instruction.address as u32),
            0xD =>  /* MULTI */ self.set_reg(instruction.reg2, self.get_reg(instruction.reg2) * instruction.address as u32),
            0xE =>  /* DIVI */ self.set_reg(instruction.reg2, self.get_reg(instruction.reg2) / instruction.address as u32),
            0xF =>  /* LDI */ self.set_reg(instruction.reg2, instruction.address as u32),
            0x11 => /* SLTI */ {
                if self.get_reg(instruction.reg1) < instruction.address as u32{
                    self.set_reg(instruction.reg3, 1);
                }
                else{
                    self.set_reg(instruction.reg3, 0);
                }
            }
            0x15 => /* BEQ */ {
                if self.get_reg(instruction.reg1) == self.get_reg(instruction.reg2) {
                    self.branch(instruction.address);
                }
            },
            0x16 => /* BNE */ {
                if self.get_reg(instruction.reg1) != self.get_reg(instruction.reg2) {
                    self.branch(instruction.address);
                }
            },
            0x17 => /* BEZ */ {
                if self.get_reg(instruction.reg1) == 0 {
                    self.branch(instruction.address);
                }
            },
            0x18 => /* BNZ */ {
                if self.get_reg(instruction.reg1) != 0 {
                    self.branch(instruction.address);
                }
            },
            0x19 => /* BGZ */ {
                if self.get_reg(instruction.reg1) > 0 {
                    self.branch(instruction.address);
                }
            },
            0x1A => /* BLZ */ {
                if self.get_reg(instruction.reg1) < 0 {
                    self.branch(instruction.address);
                }
            },
            _ => panic!("Execute error, invalid opcode for conditional branch or immediate instruction"),
        };
    }

    fn execute_uncond_jump(&mut self, instruction: DecodedInstruction) {
        match instruction.opcode {
            0x12 => /* HLT */ self.is_running = false,
            0x14 => /* JMP */ self.branch(instruction.address),
            _ => panic!("Execute error, invalid opcode for unconditional jump instruction"),
        };
    }

    fn execute_io(&mut self, instruction: DecodedInstruction) {
        match instruction.opcode {
            0x0 => /* RD */ {
                if self.get_reg(instruction.reg2) == 0 {
                    //self.set_reg(instruction.reg1, self.fetch(instruction.address));
                } else {
                    //self.set_reg(instruction.reg1, self.fetch(self.get_reg(instruction.reg2) as usize));
                }
            },
            0x1 => /* WR */ {
                if self.get_reg(instruction.reg2) == 0 {
                    //self.memory.as_ref().borrow_mut().write_to(instruction.address as usize, self.get_reg(instruction.reg1));
                } else {
                    //self.memory.as_ref().borrow_mut().write_to(self.get_reg(instruction.reg2) as usize, self.get_reg(instruction.reg1));
                }
            },
            _ => panic!("Execute error, invalid opcode for I/O jump instruction"),
        };
    }

    fn branch(&mut self, destination_address: usize) {
        let mut pcb = self.pcb.as_ref().unwrap().lock().unwrap();
        pcb.program_counter = destination_address - 1;
    }

    fn get_reg(&self, reg: u8) -> u32 {
        let pcb = self.pcb.as_ref().unwrap().lock().unwrap();
        pcb.registers[reg as usize]
    }

    fn set_reg(&mut self, reg: u8, value: u32) {
        let mut pcb = self.pcb.as_ref().unwrap().lock().unwrap();
        pcb.registers[reg as usize] = value;
    }
}

enum InstructionType {
    Arithmetic = 0b00,
    CondBranchImmediate = 0b01,
    UncondJump = 0b10,
    IO = 0b11,
}

struct DecodedInstruction {
    instr_type:InstructionType,
    opcode:u8,
    reg1:u8,
    reg2:u8,
    reg3:u8,
    address:usize,
}

impl DecodedInstruction {
    pub fn new() -> DecodedInstruction{
        DecodedInstruction{
            instr_type:InstructionType::Arithmetic,
            opcode:0,
            reg1:0,
            reg2:0,
            reg3:0,
            address:0,
        }
    }
}

#[cfg(test)]
mod tests {

}
