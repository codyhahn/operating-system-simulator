pub mod driver;
pub mod loader;
pub mod long_term_scheduler;

use driver::Driver;

fn main() {
    Driver::start();
}
