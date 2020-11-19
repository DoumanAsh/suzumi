#![no_main]

use suzumi::*;

c_ffi::c_main!(rust_main);

fn rust_main(args: c_ffi::Args) -> u8 {
    let args = match cli::Cli::new(args.into_iter().skip(1)) {
        Ok(args) => args,
        Err(code) => return code,
    };

    let db = match db::Db::open("suzumi-db") {
        Ok(db) => db,
        Err(error) => {
            eprintln!("Unable to open database: {}", error);
            return 1;
        }
    };

    let assets = assets::Assets::new();

    let mut rt = match tokio::runtime::Builder::new().basic_scheduler().enable_io().max_threads(8).build() {
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
