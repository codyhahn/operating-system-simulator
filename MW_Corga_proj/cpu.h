#ifndef MWATS_PROJ_CPU
#define MWATS_PROJ_CPU

#include <string>
#include <iostream>
#include <vector>

#include "datstructures.h"

#define uint unsigned int

class CPU{
    uint programCounter; // address of the current instruction in memory
    uint currentInstruction;
    uint regs[16];       // general purpose registers

    uint* mainMemory;    // pointer to the main memory array (RAM)
    int memSize;

    bool isByteAddress;  // If this is true, the cpu will treat memory as if addresses are per-byte instead of per-word. The program from part2 is like this.
    
    // Returns a proper address for either byte or word address modes.
    uint GetAddress(uint addr); 

    uint FetchInstr(uint address);
    DecodedInstruction Decode(uint instruction);
    void Execute(DecodedInstruction instruction);

    // Sets the program counter when branching during execution.
    void BranchTo(uint address);


    public:
        CPU(uint* mainMemory, int memSize, uint startPoint); // constructor

        // This is public so it can be called from the OS later on, such as when switching processes.
        void SetPC(uint address);

        // Sets up program counter and registers according to a process (which comes from the scheduler who gets it from the pcb)
        void LoadProcess(Process process);

        // Run the data path cycle.
        // Call FetchInstr(), then Decode(), and finally Execute()
        void Cycle();

        // Ends when the process stops, either normally or by error.
        bool isRunning;
};

#endif