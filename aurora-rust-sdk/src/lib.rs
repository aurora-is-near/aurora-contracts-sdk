use std::fmt::Display;

use serde::{Deserialize, Serialize};

// TODO: Replace with proper address implementation. Using
// aurora-engine-types isn't working because it depends in
// getrandom which is not compatible with WASM.
// TODO: Serialize / Deserialize should use hex value (without quotes or 0x)
#[derive(Serialize, Deserialize)]
pub struct Address {}

impl Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Address")
    }
}
