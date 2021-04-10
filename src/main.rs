#![cfg_attr(feature = "cargo-clippy", allow(clippy::style))]

#![no_main]

use suzumi::*;

c_ffi::c_main!(rust_main);

fn rust_main(args: c_ffi::Args) -> u8 {
    let args = match cli::Cli::new(args.into_iter().skip(1)) {
        Ok(args) => args,
        Err(code) => return code,
    };

    //tracing_subscriber::fmt::init();
    #[cfg(debug_assertions)]
    rogu::set_level(rogu::Level::DEBUG);
    #[cfg(not(debug_assertions))]
    rogu::set_level(rogu::Level::INFO);

    let mut db_dir = match std::env::current_exe() {
        Ok(mut dir) => {
            dir.pop();
            dir
        },
        Err(err) => {
            rogu::warn!("Cannot access executable directory: {}", err);
            std::path::PathBuf::new()
        },
    };

    db_dir.push("suzumi-db");

    loop {
        match run(args.clone(), &db_dir) {
            0 => match IS_SHUTDOWN.load(Ordering::Acquire) {
                true => {
                    rogu::info!("Shutting down...");
                    break 0;
                },
                false => rogu::info!("Restarting..."),
            },
            error => {
                rogu::error!("Failed with error: {}", error);
                break error;
            }
        }
    }
}

fn run(args: cli::Cli, db: &std::path::Path) -> u8 {
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
