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
mod backup_restore_tests;
#[cfg(test)]
mod test_flow;
#[cfg(test)]
mod visual_fixture_tests;

pub use error::{AppError, AppResult};
pub use state::AppState;
