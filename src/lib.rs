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

pub fn run(args: cli::Cli, db: &std::path::Path) -> u8 {
    let db = match db::Db::open(db) {
        Ok(db) => db,
        Err(error) => {
            eprintln!("Unable to open database: {}", error);
            return 1;
        }
    };

    let assets = assets::Assets::new();

    let rt = match tokio::runtime::Builder::new_current_thread().enable_io().enable_time().worker_threads(8).build() {
        Ok(rt) => rt,
        Err(error) => {
            eprintln!("Unable to start IO loop: {}", error);
            return 1;
        }
    };

    let discord = match rt.block_on(discord::Discord::new(args, db.view(), assets)) {
        Ok(discord) => discord,
        Err(code) => return code,
    };

    rt.block_on(discord.start());

    0
}
