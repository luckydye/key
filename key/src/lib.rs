mod key;

pub use key::*;

#[cfg(not(target_arch = "wasm32"))]
#[cfg(feature = "cli")]
pub mod db;

#[cfg(feature = "wasm")]
pub mod wasm;
