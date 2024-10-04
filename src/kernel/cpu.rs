use super::{process_control_block::ProcessControlBlock, Memory};

/// Controls the execution of program instructions.
pub struct CPU {
    registers:[u32; 16],
    memory:Memory,
    process_control:ProcessControlBlock,

    is_running:bool,
}

impl CPU {
    pub fn start(&mut self, process_control:ProcessControlBlock) {
        // Initialize registers to 0
        for mut reg in self.registers{
            reg = 0;
        }

        self.is_running = true;

        self.set_program_counter(process_control.mem_start_address);
    }

    /// Takes a given 32-bit integer and extracts bits from it.
    /// Used to evaluate instructions.  
    fn extract_bits(number:u32, start_index:u32, length:u32) -> u32{
        (number << start_index) >> (32 - length)
    }

    fn fetch(&self, address:usize) -> u32{
        self.memory.read_from(address.try_into().unwrap())
    }

    fn decode(&self, instruction:u32) -> DecodedInstruction {
        let mut result = DecodedInstruction::new();
        
        // Get the instruction type from the first two bits
        result.instr_type = match CPU::extract_bits(instruction, 0, 2){
            0b00 => InstructionType::Arithmetic,
            0b01 => InstructionType::CondBranchImmediate,
            0b10 => InstructionType::UncondJump,
            0b11 => InstructionType::IO,
            _ => panic!("Uh-oh! Instruction type is invalid."),
        };

        // Opcode comes from the next six bits
        result.opcode = CPU::extract_bits(instruction, 2, 6).try_into().unwrap();

        match result.instr_type{
            InstructionType::Arithmetic => {
                // Arithmetic instruction uses three registers
                result.reg1 = CPU::extract_bits(instruction, 8, 4).try_into().unwrap();
                result.reg2 = CPU::extract_bits(instruction, 12, 4).try_into().unwrap();
                result.reg3 = CPU::extract_bits(instruction, 16, 4).try_into().unwrap();
            },
            InstructionType::CondBranchImmediate => {
                // Conditional branch instructions use two registers and an address
                // Immediate instructions use two registers and a piece of data.
                result.reg1 = CPU::extract_bits(instruction, 8, 4).try_into().unwrap();
                result.reg2 = CPU::extract_bits(instruction, 12, 4).try_into().unwrap();
                result.address = CPU::extract_bits(instruction, 16,16).try_into().unwrap();
            },
            InstructionType::UncondJump => {
                // Unconditional jump only needs one address, no registers.
                result.address = CPU::extract_bits(instruction, 8, 16).try_into().unwrap();
            },
            InstructionType::IO => {
                // Input/output needs two registers and one address
                result.reg1 = CPU::extract_bits(instruction, 8, 4).try_into().unwrap();
                result.reg2 = CPU::extract_bits(instruction, 12, 4).try_into().unwrap();
                result.address = CPU::extract_bits(instruction, 16, 16).try_into().unwrap();
            },
        }

        result
    }

    fn branch(&mut self, destination_address:usize){
        // TODO - this should talk to memory somehow to make sure it's not out of bounds
        if destination_address < self.process_control.mem_start_address || destination_address > self.process_control.mem_end_address{
            panic!("Branch error, address {destination_address} is not accessible to current process.");
        }
        self.process_control.program_counter = destination_address - 1;
    }

    pub fn set_program_counter(&mut self, destination_address:usize){
        
        if destination_address < self.process_control.mem_start_address || destination_address > self.process_control.mem_end_address{
            panic!("Cannot set program counter, address {destination_address} is not accessible to current process.");
        }

        self.process_control.program_counter = destination_address;
    }

    pub fn cycle(&mut self){
        let current_instruction = self.fetch(self.process_control.program_counter);

        let current_decoded = self.decode(current_instruction);

        self.execute(current_decoded);

        self.set_program_counter(self.process_control.program_counter + 1);
    }

    fn execute(&mut self, instruction:DecodedInstruction){

        if instruction.opcode == 0x13{
            // NO-OP

            // Instructions did not specify an instruction type for this one, so I just put it here.
            return;
        }

        // Execute the instruction
        match instruction.instr_type{
            InstructionType::Arithmetic => {
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
                    _ => panic!("Execute error, invalid opcode for arithmetic instruction."),
                };
            },
            InstructionType::CondBranchImmediate => {
                match instruction.opcode{
                    0x2 => /*ST (write contents of a register into memory)*/ {

                        // This is the same as WR
                        if self.get_reg(instruction.reg2) == 0{
                            // If reg2 is 0, use the address

                            // TEMPORARY - when we implement DMA, change this to use the DMA
                            self.memory.write_to(instruction.address as usize, self.get_reg(instruction.reg1));
                        }
                        else{
                            // If reg2 is nonzero, use it as a pointer

                            // TEMPORARY - when we implement DMA, change this to use the DMA
                            self.memory.write_to(self.get_reg(instruction.reg2) as usize, self.get_reg(instruction.reg1));
                        }
                    },
                    0x3 => /*LW (read from memory to a register*/ {

                        // This is the same as RD
                        if self.get_reg(instruction.reg2) == 0{
                            // If reg2 is 0, use the address

                            // TEMPORARY - when we implement DMA, change this to use the DMA
                            self.set_reg(instruction.reg1, self.fetch(instruction.address));
                        }
                        else{
                            // If reg2 is nonzero, use it as a pointer

                            // TEMPORARY - when we implement DMA, change this to use the DMA
                            self.set_reg(instruction.reg1, self.fetch(self.get_reg(instruction.reg2) as usize));
                        }
                    },

                    // Immediate instructions
                    0xB => /*MOVI*/ self.set_reg(instruction.reg2, instruction.address as u32),
                    0xC => /*ADDI*/ self.set_reg(instruction.reg2, self.get_reg(instruction.reg2) + instruction.address as u32),
                    0xD => /*MULTI*/ self.set_reg(instruction.reg2, self.get_reg(instruction.reg2) * instruction.address as u32),
                    0xE => /*DIVI*/ self.set_reg(instruction.reg2, self.get_reg(instruction.reg2) / instruction.address as u32),
                    0xF => /*LDI*/ self.set_reg(instruction.reg2, instruction.address as u32),
                    0x11 => /*SLTI*/ {
                        if self.get_reg(instruction.reg1) < instruction.address as u32{
                            self.set_reg(instruction.reg3, 1);
                        }
                        else{
                            self.set_reg(instruction.reg3, 0);
                        }
                    }

                    // Conditional branch instructions
                    0x15 => /*BEQ*/ 
                        if self.get_reg(instruction.reg1) == self.get_reg(instruction.reg2) {
                            self.branch(instruction.address);
                        },
                    0x16 => /*BNE*/ 
                        if self.get_reg(instruction.reg1) != self.get_reg(instruction.reg2) {
                            self.branch(instruction.address);
                        },
                    0x17 => /*BEZ*/ 
                        if self.get_reg(instruction.reg1) == 0 {
                            self.branch(instruction.address);
                        },
                    0x18 => /*BNZ*/ 
                        if self.get_reg(instruction.reg1) != 0 {
                            self.branch(instruction.address);
                        },
                    0x19 => /*BGZ*/ 
                        if self.get_reg(instruction.reg1) > 0 {
                            self.branch(instruction.address);
                        },
                    0x1A => /*BLZ*/ 
                        if self.get_reg(instruction.reg1) < 0 {
                            self.branch(instruction.address);
                        },
                    _ => panic!("Execute error, invalid opcode for conditional branch or immediate instruction."),
                };
            },
            InstructionType::UncondJump => {
                match instruction.opcode{
                    0x12 => /*HLT*/ self.is_running = false,
                    0x14 => /*JMP*/ self.branch(instruction.address),
                    _ => panic!("Execute error, invalid opcode for unconditional jump instruction."),
                };
            },
            InstructionType::IO => {
                match instruction.opcode{
                    0x0 => /*RD*/ {

                        // This is the same as LW
                        if self.get_reg(instruction.reg2) == 0{
                            // If reg2 is 0, use the address

                            // TEMPORARY - when we implement DMA, change this to use the DMA
                            self.set_reg(instruction.reg1, self.fetch(instruction.address));
                        }
                        else{
                            // If reg2 is nonzero, use it as a pointer

                            // TEMPORARY - when we implement DMA, change this to use the DMA
                            self.set_reg(instruction.reg1, self.fetch(self.get_reg(instruction.reg2) as usize));
                        }

                    },
                    0x1 => /*WR*/ {

                        // This is the same as ST
                        if self.get_reg(instruction.reg2) == 0{
                            // If reg2 is 0, use the address

                            // TEMPORARY - when we implement DMA, change this to use the DMA
                            self.memory.write_to(instruction.address as usize, self.get_reg(instruction.reg1));
                        }
                        else{
                            // If reg2 is nonzero, use it as a pointer

                            // TEMPORARY - when we implement DMA, change this to use the DMA
                            self.memory.write_to(self.get_reg(instruction.reg2) as usize, self.get_reg(instruction.reg1));
                        }
                    },
                    _ => panic!("Execute error, invalid opcode for I/O jump instruction."),
                };
            },
        }
    }

    fn set_reg(&mut self, reg:u8, value:u32){
        self.registers[reg as usize] = value;
    }
    fn get_reg(&self, reg:u8)->u32{
        self.registers[reg as usize]
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
