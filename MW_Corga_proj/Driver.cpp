// Matthew Watson
// Prof. Patrick Bobbie
// Computer Organization and Architecture
// 21 April 2024
// VM Project Part 4

// This is the minimal OS driver program. It contains instances of classes representing OS components, and puts them to work running the program.

/*
REPRESENTATIONS OF OS COMPONENTS
Disk: array of strings, created in main()
RAM: array of 32 bit ints, created in main()
CPU: represented by CPU class. Contains fetch, decode, execute data path cycle.
PCB: std::vector of Processes (Process being a struct which contains the necessary variables, defined in datstructures.h)
Long Term Scheduler: The function of the LTS is carried out by the LoadDiskToRAM() method in the Scheduler class.
Short Term Scheduler: The STS is represented by the SelectNextProcess() method in the Scheduler class.
Dispatcher: The function of the dispatcher is completed by the CPU::LoadProcess() method.
Ready Queue: A queue of ints corresponding to PCB indexes. Contained within the Scheduler class.
*/

#include <iostream>
#include <fstream>
#include <string>
#include <vector>

#include "cpu.h"
#include "scheduler.h"
#include "datstructures.h"

// I'm using a lot of unsigned ints, so I thought I'd define a shorthand
#define uint unsigned int

// Print a range of values from memory.
// For each line, it will print the line number (in hex) and the contents in both decimal and hex.
void PrintMem(uint* memory, int startIndex, int linesToPrint, int memSize);

// Initialize all values in memory to zero.
void InitMem(uint* memory, int memSize);

// Initialize all values of disk to the empty string.
void InitDisk(std::string* disk, int diskSize);

// Loads values into memory from a file.
// If there's a file error, it will return false and main() will shut the program down.
bool LoadFile(std::string* memory, std::string filename);

int main(int argc, char* argv[]){
    // givenProgram.txt contains the hex instructions from part 2 of the project.
    std::string programFileName = "givenProgram.txt";

    // If an argument is specified, we'll read that into ram instead.
    if(argc > 1){
        programFileName = argv[1];
    }

    // RAM is just an array of 32 bit numbers. Disk is an array of strings.
    const int ramSize = 1024;
    const int diskSize = 2048;
    uint* ram = new uint[ramSize];
    std::string* disk = new std::string[diskSize];

    // Process control block is an array of Process structs
    std::vector<Process> pcb;

    // Initialize memory and disk to default values
    InitMem(ram, ramSize);
    InitDisk(disk, diskSize);

    // Load the given file onto the "disk"
    bool fileLoadSuccess = LoadFile(disk, programFileName);

    if(!fileLoadSuccess){
        // file load error
        delete ram;
        delete disk;
        return 1;
    }

    // CPU object
    CPU* mainCPU = new CPU(ram, ramSize, 0);

    // Scheduler object. Its constructor calls the method that performs the 'long term' scheduler operation.
    Scheduler* scheduler = new Scheduler(ram, disk, ramSize, diskSize, pcb);

    // For this project, we only have one thing to schedule from disk to RAM.
    // So, we only need to call this once.
    scheduler->LoadDiskToRAM(0);


    // Main loop. The CPU will cycle until the scheduler runs out of processes.
    while(scheduler->HasNextProcess()){
        scheduler->SelectNextProcess(mainCPU);

        while(mainCPU->isRunning){
            mainCPU->Cycle();
        }
        scheduler->RemoveCompletedProcess();
    }

    // Print the first 50 words of memory. This is sufficient to show the instructions, the data, and the output for our test program.
    // (output is on line 0x2B)
    PrintMem(ram, 0, 50, ramSize);

    // Clean up
    delete mainCPU;
    delete ram;
    delete disk;
    delete scheduler;
}

void PrintMem(uint* memory, int startIndex, int linesToPrint, int memSize){
    for(int i = 0; i < linesToPrint; i++){
        if(i >= memSize){ break; } // Prevent overflow

        // Print out the contents of the memory one line at a time.
        std::cout << std::hex << (i + startIndex) << " " << std::dec << memory[i] << " " << std::hex << memory[i] << std::endl;
    }
}

// Initialize all elements in memory to 0
void InitMem(uint* memory, int memSize){
    for(int i = 0; i < memSize; i++){
        memory[i] = 0;
    }
}

// Initialize all elements on disk to the empty string
void InitDisk(std::string* disk, int diskSize){
    for(int i = 0; i < diskSize; i++){
        disk[i] = "";
    }
}

// Load a file to disk. 
bool LoadFile(std::string* disk, std::string filename){
    std::string currentLine;

    std::ifstream readFile(filename);

    if(!readFile.good()){
        std::cout << "File \"" << filename << "\" not found." << std::endl;
        readFile.close();
        return false;
    }

    int i = 0;
    while(std::getline(readFile, currentLine)){
        disk[i] = currentLine;

        i++;
    }

    readFile.close();

    return true;
}