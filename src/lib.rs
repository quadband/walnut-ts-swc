#![deny(clippy::all)]

use crate::transform::WalnutHandler;

mod transform;
mod scan_first;

#[macro_use]
extern crate napi_derive;

#[napi]
pub fn get_handler(code: String, id: String, walnut_key: String) -> WalnutHandler {
    WalnutHandler::new(code, id, walnut_key)
}
