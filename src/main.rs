#![no_main]

#[macro_use]
mod utils;
mod data;
mod assets;
mod db;

c_ffi::c_main!(rust_main);

fn rust_main(args: c_ffi::Args) -> u8 {
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
