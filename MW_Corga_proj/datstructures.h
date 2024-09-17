#ifndef MWATS_PROJ_DATSTRUCTS
#define MWATS_PROJ_DATSTRUCTS

#include <iostream>

#define uint unsigned int

// Used in DecodedInstruction to save which type it is.
enum instrType {arithmetic = 0b00, cond_branch_immediate = 0b01, uncond_jump = 0b10, in_out = 0b11};

// Struct to hold the data for an instruction after it has been decoded
struct DecodedInstruction{
    instrType type;
    unsigned char opcode;
    unsigned char reg1 = 0, reg2 = 0, reg3 = 0;     // not all registers will be used with every instruction
    unsigned short address = 0;                     // if the instruction includes a memory address (or raw data), it'll be here

    // Ensures the instruction is valid. 
    // Will return a nonzero number if:
    //  the address is bigger than main memory
    //  a register does not exist (there are only 16 regs)
    int VerifyInstruction(int memSize, int regCount){
        // improper address
        if(address < 0 || address >= memSize){
            return 4;
        }

        // register does not exist
        if(reg1 >= regCount){
            return 1;
        }

        if(reg2 >= regCount){
            return 2;
        }

        if(reg3 >= regCount){
            return 3;
        }

        // Instruction is valid
        return 0;
    }

    // Used to debug. I would print the instruction to make sure it had decoded properly.
    void PrintInstruction(){
        std::cout << "Type: " << (int)type << " Opcode: " << std::hex << (int)opcode;

        // Using bits to signify which registers are included in the instruction
        unsigned char printBitMask = 0;

        switch (type)
        {
        case instrType::arithmetic:
            printBitMask = 0b1110;
            break;
        case instrType::in_out:
        case instrType::cond_branch_immediate:
            printBitMask = 0b1101;
            break;
        case instrType::uncond_jump:
            printBitMask = 0b0001;
            break;
        default:
            break;
        }

        if(printBitMask & 0b1000){
            std::cout << " Reg1: " << (int)reg1;
        }
        if(printBitMask & 0b0100){
            std::cout << " Reg2: " << (int)reg2;
        }
        if(printBitMask & 0b0010){
            std::cout << " Reg3: " << (int)reg3;
        }
        if(printBitMask & 0b0001){
            std::cout << " Address/Data: " << address;
        }
        std::cout << std::endl;
    }
};

// Struct to hold information about a process. The Process Control Block is a list of these.
struct Process{
    uint programCounter = 0;
    uint registers[16];

    uint startPoint = 0, dataStartPoint = 0, endPoint = 0;
};

#endif