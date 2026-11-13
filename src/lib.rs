//! Anzan (暗算): a terminal trainer for mental math.
//!
//! This library exists to serve the `anzan` binary and to keep its parts
//! testable. It is not a stable API: anything may change between
//! releases, whatever the version number says.

pub mod answer;
pub mod app;
pub mod cli;
pub mod config;
pub mod drill;
pub mod export;
pub mod expr;
pub mod generate;
pub mod history;
pub mod number;
pub mod problem;
pub mod rational;
pub mod stats;
pub mod store;
pub mod theme;
pub mod ui;
