#![no_main]

#[macro_use]
mod utils;
mod cli;
mod data;
mod assets;
mod db;

c_ffi::c_main!(rust_main);

fn rust_main(args: c_ffi::Args) -> u8 {
    let args = match cli::Cli::new(args.into_iter().skip(1)) {
        Ok(args) => args,
        Err(code) => return code,
    };
    println!("{:?}", args);
    return 1;

    let db = match db::Db::open("suzumi-db") {
        Ok(db) => db,
        Err(error) => {
            eprintln!("Unable to open database");
            return 1;
        }
    };

    let assets = assets::Assets::new();

    0
}

