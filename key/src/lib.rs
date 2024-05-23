mod key;

pub use key::*;

#[cfg(not(target_arch = "wasm32"))]
pub mod db;

#[cfg(target_arch = "wasm32")]
pub mod wasm;
