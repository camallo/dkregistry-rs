extern crate dkregistry;
extern crate tokio_core;

use self::tokio_core::reactor::Core;

static REGISTRY: &'static str = "gcr.io";

fn get_env() -> Option<(String, String)> {
    let user = ::std::env::var("DKREG_GCR_USER");
    let password = ::std::env::var("DKREG_GCR_PASSWD");
    match (user, password) {
        (Ok(u), Ok(t)) => Some((u, t)),
        _ => None,
    }
}

#[test]
fn test_dockerio_getenv() {
    if get_env().is_none() {
        println!("[WARN] {}: missing DKREG_GCR_USER / DKREG_GCR_PASSWD",
                 REGISTRY);
    }
}

#[test]
fn test_gcrio_base() {
    let (user, password) = match get_env() {
        Some(t) => t,
        None => return,
    };

    let mut tcore = Core::new().unwrap();
    let dclient = dkregistry::v2::Client::configure(&tcore.handle())
        .registry(REGISTRY)
        .insecure_registry(false)
        .username(Some(user))
        .password(Some(password))
        .build()
        .unwrap();

    let futcheck = dclient.is_v2_supported().unwrap();

    let res = tcore.run(futcheck).unwrap();
    assert_eq!(res, true);
}

#[test]
fn test_gcrio_insecure() {
    let mut tcore = Core::new().unwrap();
    let dclient = dkregistry::v2::Client::configure(&tcore.handle())
        .registry(REGISTRY)
        .insecure_registry(true)
        .username(None)
        .password(None)
        .build()
        .unwrap();

    let futcheck = dclient.is_v2_supported().unwrap();

    let res = tcore.run(futcheck).unwrap();
    assert_eq!(res, false);
}
