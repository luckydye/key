use wasm_bindgen::prelude::*;

use crate::key;

#[wasm_bindgen]
extern "C" {
  #[wasm_bindgen(js_namespace = console)]
  fn log(s: &str);
}

macro_rules! log {
    ($($t:tt)*) => (log(&format!("[wasm] {}", format_args!($($t)*).to_string())))
}

#[wasm_bindgen]
pub async fn greet() {
  log!("Hello, hello-wasm!");
}
