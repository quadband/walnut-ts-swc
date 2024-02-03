#![deny(clippy::all)]

use crate::transform::WalnutHandler;

mod transform;
mod scan_first;
mod helpers;
mod finalize;
mod resolver;
mod resolve_modules;

use crate::resolve_modules::resolve_deps;

#[macro_use]
extern crate napi_derive;

#[napi]
pub fn get_handler(code: String, id: String, walnut_key: String) -> WalnutHandler {
    WalnutHandler::new(code, id, walnut_key)
}

#[napi]
pub fn resolve_dependencies(base: String, entry: String) {
    resolve_deps(&base, &entry);
}