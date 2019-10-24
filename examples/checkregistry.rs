extern crate dkregistry;
extern crate tokio;

use std::{boxed, error};
use tokio::runtime::current_thread::Runtime;

fn main() {
    let registry = match std::env::args().nth(1) {
        Some(x) => x,
        None => "registry-1.docker.io".into(),
    };

    let res = run(&registry);

    if let Err(e) = res {
        println!("[{}] {}", registry, e);
        std::process::exit(1);
    };
}

fn run(host: &str) -> Result<bool, boxed::Box<dyn error::Error>> {
    let mut runtime = Runtime::new()?;
    let dclient = try!(dkregistry::v2::Client::configure()
        .registry(host)
        .insecure_registry(false)
        .build());
    let futcheck = dclient.is_v2_supported();

    let supported = runtime.block_on(futcheck)?;
    if supported {
        println!("{} supports v2", host);
    } else {
        println!("{} does NOT support v2", host);
    }
    Ok(supported)
}
