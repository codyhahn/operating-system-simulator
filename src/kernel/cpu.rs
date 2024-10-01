use super::Memory;

/// Controls the execution of program instructions.
pub struct CPU {
    program_counter:u32,
    current_instruction:u32,
    registers:[u32; 16],
    memory:Memory,

    is_running:bool,
}

impl CPU {
    pub fn start(&mut self, starting_address:u32) {
        // Initialize registers to 0
        for mut reg in self.registers{
            reg = 0;
        }

        self.is_running = true;

        self.set_program_counter(starting_address);
    }

    /*  Takes a given 32-bit integer and extracts bits from it.
        Used to evaluate instructions. */  
    fn extract_bits(number:u32, start_index:u32, length:u32) -> u32{
        (number << start_index) >> (32 - length)
    }

    fn fetch(&self, address:u32) -> u32{
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

    fn execute(&self, instruction:DecodedInstruction){
        // Execute the instruction
    }

    fn branch(&mut self, destination_address:u32){
        // TODO - this should talk to memory somehow to make sure it's not out of bounds
        self.program_counter = destination_address - 1;
    }

    pub fn set_program_counter(&mut self, destination_address:u32){
        
        // TODO - check for out of bounds

        self.program_counter = destination_address;
    }

    pub fn cycle(&mut self){
        self.current_instruction = self.fetch(self.program_counter);

        let current_decoded = self.decode(self.current_instruction);

        self.execute(current_decoded);

        self.set_program_counter(self.program_counter + 1);
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
    address:u16,
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
