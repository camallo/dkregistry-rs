extern crate dkregistry;
extern crate tokio_core;

use std::{error, boxed};
use tokio_core::reactor::Core;

type Result<T> = std::result::Result<T, boxed::Box<error::Error>>;

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

fn run(host: &str) -> Result<bool> {
    let mut tcore = try!(Core::new());
    let dclient = try!(dkregistry::v2::Client::configure(&tcore.handle())
        .registry(host)
        .insecure_registry(false)
        .build());
    let futcheck = try!(dclient.is_v2_supported());

    let supported = try!(tcore.run(futcheck));
    match supported {
        false => println!("{} does NOT support v2", host),
        true => println!("{} supports v2", host),
    }
    return Ok(supported);
}
