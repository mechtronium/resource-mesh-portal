#[macro_use]
extern crate anyhow;

#[macro_use]
extern crate lazy_static;

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use crate::version::latest::log::Log;

pub mod version;
pub mod message;

pub fn std_logger(log: Log ) {
    match log {
        Log::Warn(m) => {
            println!("WARN: {}", m);
        }
        Log::Info(m) => {
            println!("INFO: {}", m);
        }
        Log::Error(m) => {
            eprintln!("ERROR: {}", m);
        }
        Log::Fatal(m) => {
            eprintln!("FATAL: {}", m);
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
    }
}
