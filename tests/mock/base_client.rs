extern crate dkregistry;
extern crate mockito;
extern crate tokio_core;

use self::mockito::mock;
use self::tokio_core::reactor::Core;

static API_VERSION_K: &'static str = "Docker-Distribution-API-Version";
static API_VERSION_V: &'static str = "registry/2.0";

fn mock_version_ep() {
    mock("GET", "/v2/").with_status(200).with_header(API_VERSION_K, API_VERSION_V).create();
}

#[test]
fn test_base_no_insecure() {
    mock_version_ep();

    let mut tcore = Core::new().unwrap();
    let dclient = dkregistry::v2::Client::configure(&tcore.handle())
        .registry(mockito::SERVER_ADDRESS)
        .insecure_registry(false)
        .username(None)
        .password(None)
        .build()
        .unwrap();

    let futcheck = dclient.is_v2_supported().unwrap();

    // This relies on the fact that mockito is HTTP-only and
    // trying to speak TLS to it results in garbage/errors.
    tcore.run(futcheck).is_err();

    mockito::reset();
}

#[test]
fn test_base_useragent() {
    mock("GET", "/v2/")
        .match_header("user-agent", dkregistry::USER_AGENT)
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
    assert_eq!(res, true);

    mockito::reset();
}

#[test]
fn test_base_custom_useragent() {
    let ua = "custom-ua/1.0";

    mock("GET", "/v2/")
        .match_header("user-agent", ua)
        .with_status(200)
        .with_header(API_VERSION_K, API_VERSION_V)
        .create();

    let mut tcore = Core::new().unwrap();
    let dclient = dkregistry::v2::Client::configure(&tcore.handle())
        .registry(mockito::SERVER_ADDRESS)
        .insecure_registry(true)
        .user_agent(Some(ua.to_string()))
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
fn test_base_no_useragent() {
    mock("GET", "/v2/")
        .match_header("user-agent", mockito::Matcher::Missing)
        .with_status(200)
        .with_header(API_VERSION_K, API_VERSION_V)
        .create();

    let mut tcore = Core::new().unwrap();
    let dclient = dkregistry::v2::Client::configure(&tcore.handle())
        .registry(mockito::SERVER_ADDRESS)
        .insecure_registry(true)
        .user_agent(None)
        .username(None)
        .password(None)
        .build()
        .unwrap();

    let futcheck = dclient.is_v2_supported().unwrap();

    let res = tcore.run(futcheck).unwrap();
    assert_eq!(res, true);

    mockito::reset();
}
