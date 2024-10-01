mod io;
mod kernel;

use kernel::Driver;

fn main() {
    let mut _driver = Driver::new();
    _driver.start();
}