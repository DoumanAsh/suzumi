#![cfg_attr(feature = "cargo-clippy", allow(clippy::style))]

pub use core::sync::atomic::{AtomicBool, Ordering};

#[macro_use]
pub mod utils;
pub mod cli;
pub mod data;
pub mod assets;
pub mod db;
pub mod game;
pub mod discord;

pub static IS_SHUTDOWN: AtomicBool = AtomicBool::new(false);
