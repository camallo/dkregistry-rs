extern crate dkregistry;
extern crate futures;
extern crate tokio_core;

mod common;

use futures::prelude::*;
use std::result::Result;
use std::{boxed, error};
use tokio_core::reactor::Core;

fn main() {
    let registry = match std::env::args().nth(1) {
        Some(x) => x,
        None => "registry-1.docker.io".into(),
    };

    let user = std::env::var("DKREG_USER").ok();
    if user.is_none() {
        println!("[{}] no $DKREG_USER for login user", registry);
    }
    let password = std::env::var("DKREG_PASSWD").ok();
    if password.is_none() {
        println!("[{}] no $DKREG_PASSWD for login password", registry);
    }

    let res = run(&registry, user, password);

    if let Err(e) = res {
        println!("[{}] {}", registry, e);
        std::process::exit(1);
    };
}

fn run(
    host: &str,
    user: Option<String>,
    passwd: Option<String>,
) -> Result<(), boxed::Box<error::Error>> {
    let mut tcore = Core::new()?;

    let mut client = dkregistry::v2::Client::configure()
        .registry(host)
        .insecure_registry(false)
        .username(user)
        .password(passwd)
        .build()?;

    let login_scope = "";

    let futures = common::authenticate_client(&mut client, &login_scope)
        .and_then(|dclient| dclient.is_v2_supported());

    match tcore.run(futures) {
        Ok(login_successful) if login_successful => Ok(()),
        Err(e) => Err(Box::new(e)),
        _ => Err("Login unsucessful".into()),
    }
}
