extern crate dkregistry;
extern crate mockito;
extern crate tokio_core;

use self::mockito::mock;
use self::tokio_core::reactor::Core;

static API_VERSION_K: &'static str = "Docker-Distribution-API-Version";
static API_VERSION_V: &'static str = "registry/2.0";

fn mock_version_ep() {
    mock("GET", "/v2/")
        .with_status(200)
        .with_header(API_VERSION_K, API_VERSION_V)
        .create();
}

#[test]
fn test_version_check_status_ok() {
    mock_version_ep();

    let mut tcore = Core::new().unwrap();
    let dclient = dkregistry::v2::Client::configure(&tcore.handle())
        .registry(mockito::SERVER_ADDRESS)
        .insecure_registry(true)
        .username(None)
        .password(None)
        .build()
        .unwrap();

    let futcheck = dclient.is_v2_supported().unwrap();

    let res = tcore.run(futcheck).unwrap();
    assert_eq!(res, true);

    mockito::reset();
}

#[test]
fn test_version_check_status_unauth() {
    mock("GET", "/v2/")
        .with_status(401)
        .with_header(API_VERSION_K, API_VERSION_V)
        .create();

    let mut tcore = Core::new().unwrap();
    let dclient = dkregistry::v2::Client::configure(&tcore.handle())
        .registry(mockito::SERVER_ADDRESS)
        .insecure_registry(true)
        .username(None)
        .password(None)
        .build()
        .unwrap();

    let futcheck = dclient.is_v2_supported().unwrap();

    let res = tcore.run(futcheck).unwrap();
    assert_eq!(res, true);

    mockito::reset();
}

#[test]
fn test_version_check_status_notfound() {
    mock("GET", "/v2/")
        .with_status(404)
        .with_header(API_VERSION_K, API_VERSION_V)
        .create();

    let mut tcore = Core::new().unwrap();
    let dclient = dkregistry::v2::Client::configure(&tcore.handle())
        .registry(mockito::SERVER_ADDRESS)
        .insecure_registry(true)
        .username(None)
        .password(None)
        .build()
        .unwrap();

    let futcheck = dclient.is_v2_supported().unwrap();

    let res = tcore.run(futcheck).unwrap();
    assert_eq!(res, false);

    mockito::reset();
}

#[test]
fn test_version_check_status_forbidden() {
    mock("GET", "/v2/")
        .with_status(403)
        .with_header(API_VERSION_K, API_VERSION_V)
        .create();

    let mut tcore = Core::new().unwrap();
    let dclient = dkregistry::v2::Client::configure(&tcore.handle())
        .registry(mockito::SERVER_ADDRESS)
        .insecure_registry(true)
        .username(None)
        .password(None)
        .build()
        .unwrap();

    let futcheck = dclient.is_v2_supported().unwrap();

    let res = tcore.run(futcheck).unwrap();
    assert_eq!(res, false);

    mockito::reset();
}

#[test]
fn test_version_check_noheader() {
    mock("GET", "/v2/")
        .with_status(403)
        .create();

    let mut tcore = Core::new().unwrap();
    let dclient = dkregistry::v2::Client::configure(&tcore.handle())
        .registry(mockito::SERVER_ADDRESS)
        .insecure_registry(true)
        .username(None)
        .password(None)
        .build()
        .unwrap();

    let futcheck = dclient.is_v2_supported().unwrap();

    let res = tcore.run(futcheck).unwrap();
    assert_eq!(res, false);

    mockito::reset();
}

#[test]
fn test_version_check_trailing_slash() {
    mock("GET", "/v2")
        .with_status(200)
        .with_header(API_VERSION_K, API_VERSION_V)
        .create();

    let mut tcore = Core::new().unwrap();
    let dclient = dkregistry::v2::Client::configure(&tcore.handle())
        .registry(mockito::SERVER_ADDRESS)
        .insecure_registry(true)
        .username(None)
        .password(None)
        .build()
        .unwrap();

    let futcheck = dclient.is_v2_supported().unwrap();

    let res = tcore.run(futcheck).unwrap();
    assert_eq!(res, false);

    mockito::reset();
}
