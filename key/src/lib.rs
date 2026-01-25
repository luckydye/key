mod key;

pub use key::*;

#[cfg(not(target_arch = "wasm32"))]
pub mod db;

#[cfg(feature = "wasm")]
pub mod wasm;
