extern crate dkregistry;
extern crate tokio_core;

use self::tokio_core::reactor::Core;

static REGISTRY: &'static str = "registry-1.docker.io";

fn check_env() -> (String, String) {
    let user = ::std::env::var("DKREG_DOCKER_USER").expect("Missing $DKREG_DOCKER_USER");
    let password = ::std::env::var("DKREG_DOCKER_PASSWD").expect("Missing DKREG_DOCKER_PASSWD");
    (user, password)
}

#[test]
fn test_dockerio_base() {
    check_env();

    let mut tcore = Core::new().unwrap();
    let dclient = dkregistry::v2::Client::configure(&tcore.handle())
        .registry(REGISTRY)
        .insecure_registry(false)
        .username(None)
        .password(None)
        .build()
        .unwrap();

    let futcheck = dclient.is_v2_supported().unwrap();

    let res = tcore.run(futcheck).unwrap();
    assert_eq!(res, true);
}

#[test]
fn test_dockerio_insecure() {
    check_env();

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
