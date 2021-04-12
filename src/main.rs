#![cfg_attr(feature = "cargo-clippy", allow(clippy::style))]

#![no_main]

use suzumi::*;

c_ffi::c_main!(rust_main);

fn await_network() {
    static PINGER: Pinger = Pinger::new(std::net::IpAddr::V4(std::net::Ipv4Addr::new(8, 8, 8, 8)));

    while let Err(_) = PINGER.ping() {
        rogu::info!("Awaiting network...");
        std::thread::sleep(core::time::Duration::from_secs(1));
    }
}

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

    await_network();

    loop {
        match run(args.clone(), &db_dir) {
            0 => match IS_SHUTDOWN.load(Ordering::Acquire) {
                true => {
                    rogu::info!("Shutting down...");
                    break 0;
                },
                false => {
                    rogu::info!("Restarting...");
                },
            },
            error => {
                rogu::error!("Failed with error: {}", error);
                await_network();
                std::thread::sleep(core::time::Duration::from_secs(10));
            }
        }
    }
}
