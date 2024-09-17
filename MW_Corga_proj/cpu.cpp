#include <iostream>
#include <vector>

#include "datstructures.h"
#include "cpu.h"

#define uint unsigned int

// Function to extract bits from a 32-bit integer.
uint GetBits(uint num, int startIndex, int length){
    uint result = (num << startIndex) >> (32 - length);
    return result;
}

CPU::CPU(uint* mainMemory, int memSize, uint startPoint){
    // Initialize all registers to 0
    for(int i = 0; i < 16; i++){
        regs[i] = 0;
    }

    // Start the program counter at the specified position
    programCounter = startPoint;

    // Cpu is running
    isRunning = true;

    this->mainMemory = mainMemory;
    this->memSize = memSize;

    // I decided to only use the address mode used in the first part of the project, for simplicity.
    this->isByteAddress = true;
}

void CPU::SetPC(uint address){
    // If address is valid, set the program counter to it.

    if(address < 0 || address >= memSize){
        std::cout << "Set PC Error, address " << address << " is out of memory bounds." << std::endl;
        isRunning = false;
        return;
    }

    programCounter = address;
}

void CPU::LoadProcess(Process process){
    // Load the registers and set the program counter according to the variables in the specified process.
    // process comes from the process control block.

    for(int i = 0; i < 16; i++){
        regs[i] = process.registers[i];
    }

    SetPC(process.programCounter);
    isRunning = true;
}

void CPU::BranchTo(uint address){
    if(address < 0 || address >= memSize){
        std::cout << "Branch Error, address " << address << " is out of memory bounds." << std::endl;
        isRunning = false;
        return;
    }

    // Set the program counter to the previous address to the one given.
    // This is because BranchTo() is always called during Execute(). After Execute(), the program counter is incremented.
    programCounter = address - 1;
}

uint CPU::GetAddress(uint address){
    if(isByteAddress){
        return address / 4;
    }
    else{
        return address;
    }
}

void CPU::Cycle(){
    // Load the instruction register.
    currentInstruction = FetchInstr(programCounter);

    // Decode the instruction, and put it in its own data structure
    DecodedInstruction currentInstrDecoded = Decode(currentInstruction);

    // Execute the decoded instruction
    Execute(currentInstrDecoded); 

    // Increment program counter to point to the next instruction.
    SetPC(programCounter + 1);

    // temporary measure to prevent infinite loops
    if(programCounter > 10000){
        isRunning = false;
    }
}

uint CPU::FetchInstr(uint address){
    return mainMemory[address];
}

DecodedInstruction CPU::Decode(uint instruction){
    DecodedInstruction result;

    // 2 bits for the instruction type
    result.type = (instrType)GetBits(instruction, 0, 2);

    // 6 bits for the opcode
    result.opcode = (unsigned char)GetBits(instruction, 2, 6);

    switch(result.type)
    {
    case instrType::arithmetic:
        // 3 registers used
        result.reg1 = (unsigned char)GetBits(instruction, 8, 4);
        result.reg2 = (unsigned char)GetBits(instruction, 12, 4);
        result.reg3 = (unsigned char)GetBits(instruction, 16, 4);
        break;
    case instrType::cond_branch_immediate:
        // 2 registers and a 16-bit address
        result.reg1 = (unsigned char)GetBits(instruction, 8, 4);
        result.reg2 = (unsigned char)GetBits(instruction, 12, 4);
        result.address = (unsigned short)GetBits(instruction, 16, 16);
        break;
    case instrType::uncond_jump:
        // one address, no registers
        result.address = (unsigned short)GetBits(instruction, 8, 16);
        break;
    case instrType::in_out:
        // 2 registers and a 16-bit address
        result.reg1 = (unsigned char)GetBits(instruction, 8, 4);
        result.reg2 = (unsigned char)GetBits(instruction, 12, 4);
        result.address = (unsigned short)GetBits(instruction, 16, 16);
        break;
    default:
        // Code should never go here, because the instruction type is 2 bits, and there are only 4 combinations, which I've covered.
        std::cout << "Decode error, invalid instruction type (somehow)." << std::endl;
        break;
    }

    return result;
}

void CPU::Execute(DecodedInstruction instr){

    // Verify the instruction is valid, at least in the sense that the registers and addresses are valid.
    int verification = instr.VerifyInstruction(memSize, 16);
    switch (verification)
    {
    case 1:
        std::cout << "Invalid Instruction Error, First register " << instr.reg1 << " does not exist." << std::endl;
        break;
    case 2:
        std::cout << "Invalid Instruction Error, Second register " << instr.reg2 << " does not exist." << std::endl;
        break;
    case 3:
        std::cout << "Invalid Instruction Error, Third register " << instr.reg3 << " does not exist." << std::endl;
        break;
    case 4:
        std::cout << "Invalid Instruction Error, Address " << instr.address << " is outside of the main memory." << std::endl;
        break;
    default:
        break;
    }

    if(verification != 0){
        isRunning = false;
        return;
    }

    switch (instr.type)
    {
    case instrType::arithmetic:
        switch (instr.opcode)
        {
        case 0x4: // MOV 
            // transfer data from reg1 into reg2.
            regs[instr.reg2] = regs[instr.reg1];
            break;
        case 0x5: // ADD
            // add register 2 and 3, store the result in register 1
            regs[instr.reg1] = regs[instr.reg2] + regs[instr.reg3];
            break;
        case 0x6: // SUB
            regs[instr.reg1] = regs[instr.reg2] - regs[instr.reg3];
            break;
        case 0x7: // MUL
            regs[instr.reg1] = regs[instr.reg2] * regs[instr.reg3];
            break;
        case 0x8: // DIV
            regs[instr.reg1] = regs[instr.reg2] / regs[instr.reg3];
            break;
        case 0x9: // AND
            regs[instr.reg1] = regs[instr.reg2] & regs[instr.reg3];
            break;
        case 0xA: // OR
            regs[instr.reg1] = regs[instr.reg2] | regs[instr.reg3];
            break;
        case 0x10: // SLT
            if(regs[instr.reg1] < regs[instr.reg2]){
                regs[instr.reg3] = 1;
            }
            else{
                regs[instr.reg3] = 0;
            }
            break;
        default:
            std::cout << "Execute error, invalid opcode for arithmetic instruction: " << std::hex << instr.opcode << std::endl;
            isRunning = false;
            break;
        }
        break;
    case instrType::cond_branch_immediate:
        switch (instr.opcode)
        {
        /*  For read/write: if register 2 is 0000 (since the accumulator won't be used as a pointer ever), use the address.
            otherwise, use register 2 as a pointer.
        */
        case 0x2: // ST (Same as WR)
            if(instr.reg2 != 0){
                mainMemory[GetAddress(regs[instr.reg2])] = regs[instr.reg1];
            }
            else{
                mainMemory[GetAddress(instr.address)] = regs[instr.reg1];
            }
            break;
        case 0x3: // LW (same as RD)
            if(instr.reg2 != 0){
                regs[instr.reg2] = mainMemory[GetAddress(regs[instr.reg1])];
            }
            else{
                regs[instr.reg2] = mainMemory[GetAddress(instr.address)];
            }
            break;
        
        // Immediates
        case 0xB: // MOVI
            regs[instr.reg2] = instr.address;
            break;
        case 0xC: // ADDI
            regs[instr.reg2] += instr.address;
            break;
        case 0xD: // MULI
            regs[instr.reg2] *= instr.address;
            break;
        case 0xE: // DIVI
            regs[instr.reg2] /= instr.address;
            break;
        case 0xF: // LDI (same as MOVI?)
            regs[instr.reg2] = instr.address;
            break;
        case 0x11: // SLTI
            if(regs[instr.reg2] < instr.address){
                regs[instr.reg1] = 1;
            }
            else{
                regs[instr.reg1] = 0;
            }
            break;

        // Conditional Branches
        case 0x15: // BEQ       branch if reg1 = reg2
            if(regs[instr.reg1] == regs[instr.reg2]){
                BranchTo(GetAddress(instr.address));
            }
            break;
        case 0x16: // BNE       branch if reg1 != reg2
            if(regs[instr.reg1] != regs[instr.reg2]){
                BranchTo(GetAddress(instr.address));
            }
            break;
        case 0x17: // BEZ       branch if reg1 = 0
            if(regs[instr.reg1] == 0){
                BranchTo(GetAddress(instr.address));
            }
            break;
        case 0x18: // BNZ       branch if reg1 != 0
            if(regs[instr.reg1] != 0){
                BranchTo(GetAddress(instr.address));
            }
            break;
        case 0x19: // BGZ       branch if reg1 > 0
            if(regs[instr.reg1] > 0){
                BranchTo(GetAddress(instr.address));
            }
            break;
        case 0x1A: // BLZ       branch if reg1 < 0
            if(regs[instr.reg1] < 0){
                BranchTo(GetAddress(instr.address));
            }
            break;
        default:
            std::cout << "Execute error, invalid opcode for conditional branch/immediate instruction: " << std::hex << instr.opcode << std::endl;
            isRunning = false;
            break;
        }
        break;
    case instrType::uncond_jump:
        switch (instr.opcode)
        {
        case 0x12: // HLT
            isRunning = false;
            std::cout << "Halting program normally." << std::endl;
            break;
        case 0x14: // JMP
            BranchTo(GetAddress(instr.address));
            // Not using SetPC(), because SetPC won't accept an address less than 0.
            // Need to set it to the address - 1, because the program counter is incremented by one at the end of every cycle.
            break;
        default:
            std::cout << "Execute error, invalid opcode for unconditional branch instruction: " << std::hex << instr.opcode << std::endl;
            break;
        }
        break;
    case instrType::in_out:
        switch (instr.opcode)
        {
        /*  For read/write: if register 2 is 0000 (since the accumulator won't be used as a pointer ever), use the address.
            otherwise, use register 2 as a pointer.
        */
        case 0x0: // RD (read)
            if(instr.reg2 != 0){
                regs[instr.reg1] = mainMemory[GetAddress(regs[instr.reg2])];
            }
            else{
                regs[instr.reg1] = mainMemory[GetAddress(instr.address)];
            }
            break;
        case 0x1: // WR (write)
            if(instr.reg2 != 0){
                mainMemory[GetAddress(regs[instr.reg2])] = regs[instr.reg1];
            }
            else{
                mainMemory[GetAddress(instr.address)] = regs[instr.reg1];
            }
            break;
        default:
            std::cout << "Execute error, invalid opcode for I/O instruction: " << std::hex << instr.opcode << std::endl;
            isRunning = false;
            break;
        }
        break;
    default:
        std::cout << "Execute error, invalid instruction type (somehow)." << std::endl;
        isRunning = false;
        break;
    }
}