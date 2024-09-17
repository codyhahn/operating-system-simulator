
// The long term and short term schedulers are bundled together in this class.
// This also contains the ready queue.

#include <string>
#include <vector>
#include <queue>
#include <iostream>

#include "datstructures.h"
#include "cpu.h"

#define uint unsigned int

class Scheduler{

    uint* ram;
    std::string* disk;
    int ramSize, diskSize;

    // Ready Queue is just a list of integers corresponding to process control block indexes
    std::queue<int> readyQueue;

    // Reference to the process control block
    std::vector<Process> pcb;    

    public:
        Scheduler(uint* ram, std::string* disk, int ramSize, int diskSize, std::vector<Process>& pcb){
            this->ram = ram;
            this->disk = disk;
            this->ramSize = ramSize;
            this->diskSize = diskSize;

            this->pcb = pcb;
        }

        // Loads a program from 'disk' to 'RAM'.
        // This method performs the "Long-term scheduler" operation.
        void LoadDiskToRAM(int startPoint){
            int diskIndex = startPoint;
            int memIndex = 0;
            bool processLoaded = false;

            std::string currentLine = "";

            // Create a process and initialize its registers
            pcb.push_back(Process());
            int curIndex = pcb.size() - 1;
            for(int i = 0; i < 16; i++){
                pcb[curIndex].registers[i] = 0;
            }

            // Counts the number of times '/' appears. Used to determine which variables to set in the process.
            int count = 0;

            while(!processLoaded){
                if(diskIndex >= diskSize){
                    std::cout << "Disk read error: attempting to load outside of disk bounds." << std::endl;
                    return;
                }

                if(disk[diskIndex][0] == '/'){
                    // Each program has three of these. JOB, DATA, and END.
                    
                    count++;

                    switch (count)
                    {
                    case 1:
                        // Set the start point and program counter in the process.
                        pcb[curIndex].startPoint = memIndex;
                        pcb[curIndex].programCounter = memIndex;
                        break;
                    case 2:
                        // Set the data start point.
                        pcb[curIndex].dataStartPoint = memIndex;
                        break;
                    case 3:
                        // After this case, we're done loading.

                        // Set the end point of the process.
                        pcb[curIndex].endPoint = memIndex;

                        // Add the index of the latest process to the ready queue.
                        readyQueue.push(curIndex);
                        processLoaded = true;
                        break;
                    
                    default:
                        break;
                    }
                }
                else{
                    if(memIndex >= ramSize){
                        std::cout << "Disk read error: attempting to write to nonexistent RAM space." << std::endl;
                        return;
                    }

                    // Load a value to RAM
                    uint intrep = (uint)(std::stoul(disk[diskIndex], 0, 16));
                    ram[memIndex] = intrep;
                    memIndex++;
                }

                diskIndex++;
            }
        }
    
        /*
        Removes the first element in the ready queue and its corresponding process in the PCB. 
        Then, it looks at the next element in the ready queue and sets up the CPU's variables according to that process.

        In other words, this method performs the "short term scheduler" and "dispatcher" operations.
        */
        void SelectNextProcess(CPU* cpu){
            // If the size is 1, we'll assume this is the first process, and set up the cpu with its variables.
            if(pcb.size() > 1){
                int lastProcess = readyQueue.front();
                readyQueue.pop();

                pcb.erase(pcb.begin() + lastProcess);
            }

            if(pcb.size() == 0 || readyQueue.size() == 0){
                return;
            }

            int newProcess = readyQueue.front();

            // Dispatch
            cpu->LoadProcess(pcb[newProcess]);
        }

        // Determine whether or not there is a job left to be done
        bool HasNextProcess(){
            return (readyQueue.size() != 0);
        }

        // Removes the most recently completed process from the process control block and the ready queue.
        void RemoveCompletedProcess(){
            int lastProcess = readyQueue.front();
            readyQueue.pop();

            pcb.erase(pcb.begin() + lastProcess);
        }
       

};