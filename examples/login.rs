extern crate dkregistry;
extern crate tokio_core;

use std::{boxed, error};
use tokio_core::reactor::Core;

type Result<T> = std::result::Result<T, boxed::Box<error::Error>>;

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

fn run(host: &str, user: Option<String>, passwd: Option<String>) -> Result<()> {
    let mut tcore = try!(Core::new());
    let dclient = try!(
        dkregistry::v2::Client::configure(&tcore.handle())
            .registry(host)
            .insecure_registry(false)
            .username(user)
            .password(passwd)
            .build()
    );

    let futcheck = try!(dclient.is_v2_supported());
    let supported = try!(tcore.run(futcheck));
    if !supported {
        return Err("API v2 not supported".into());
    }

    let futauth = try!(dclient.is_auth(None));
    let logged_in = try!(tcore.run(futauth));
    if logged_in {
        return Err("no login performed, but already authenticated".into());
    }

    let fut_token = try!(dclient.login(vec![]));
    let token = try!(tcore.run(fut_token));

    let futauth = try!(dclient.is_auth(Some(token.token())));
    let done = try!(tcore.run(futauth));

    match done {
        false => return Err("login failed".into()),
        true => println!("logged in!",),
    }
    let futcheck = try!(dclient.is_v2_supported());
    if !try!(tcore.run(futcheck)) {
        return Err("API check failed after login".into());
    };

    return Ok(());
}
