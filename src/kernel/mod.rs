mod long_term_scheduler;
mod memory;
mod process_control_block;

use long_term_scheduler::LongTermScheduler;
use memory::Memory;
use process_control_block::ProcessControlBlock;

use memory::Memory;

pub mod driver;

pub use driver::Driver;