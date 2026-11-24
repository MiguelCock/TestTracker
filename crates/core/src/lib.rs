#![deny(unsafe_code)]

pub mod ai;
pub mod bundles;
mod db;
pub mod documents;
pub mod domain;
pub mod error;
pub mod imports;
pub mod mcp;
pub mod plugins;
mod security;
pub mod services;
pub mod state;

#[cfg(test)]
mod test_flow;

pub use error::{AppError, AppResult};
pub use state::AppState;
