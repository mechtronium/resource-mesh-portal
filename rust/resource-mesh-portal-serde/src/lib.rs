#[macro_use]
extern crate anyhow;

#[macro_use]
extern crate lazy_static;

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

pub mod version;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
    }
}
