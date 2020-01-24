extern crate dkregistry;
extern crate tokio;

use self::tokio::runtime::Runtime;

static REGISTRY: &'static str = "registry-1.docker.io";

fn get_env() -> Option<(String, String)> {
    let user = ::std::env::var("DKREG_DOCKER_USER");
    let password = ::std::env::var("DKREG_DOCKER_PASSWD");
    match (user, password) {
        (Ok(u), Ok(t)) => Some((u, t)),
        _ => None,
    }
}

#[test]
fn test_dockerio_getenv() {
    if get_env().is_none() {
        println!(
            "[WARN] {}: missing DKREG_DOCKER_USER / DKREG_DOCKER_PASSWD",
            REGISTRY
        );
    }
}

#[test]
fn test_dockerio_base() {
    let (user, password) = match get_env() {
        Some(t) => t,
        None => return,
    };

    let mut runtime = Runtime::new().unwrap();
    let dclient = dkregistry::v2::Client::configure()
        .registry(REGISTRY)
        .insecure_registry(false)
        .username(Some(user))
        .password(Some(password))
        .build()
        .unwrap();

    let futcheck = dclient.is_v2_supported();

    let res = runtime.block_on(futcheck).unwrap();
    assert_eq!(res, true);
}

#[test]
fn test_dockerio_insecure() {
    let mut runtime = Runtime::new().unwrap();
    let dclient = dkregistry::v2::Client::configure()
        .registry(REGISTRY)
        .insecure_registry(true)
        .username(None)
        .password(None)
        .build()
        .unwrap();

    let futcheck = dclient.is_v2_supported();

    let res = runtime.block_on(futcheck).unwrap();
    assert_eq!(res, true);
}
