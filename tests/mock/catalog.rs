extern crate dkregistry;
extern crate mockito;
extern crate tokio_core;
extern crate futures;

use self::mockito::mock;
use self::tokio_core::reactor::Core;
use self::futures::Stream;

#[test]
fn test_catalog_simple() {
    let repos = r#"{"repositories": ["r1/i1", "r2"]}"#;

    let ep = format!("/v2/_catalog");
    mock("GET", ep.as_str())
        .with_status(200)
        .with_body(repos)
        .create();

    let mut tcore = Core::new().unwrap();
    let dclient = dkregistry::v2::Client::configure(&tcore.handle())
        .registry(mockito::SERVER_ADDRESS)
        .insecure_registry(true)
        .username(None)
        .password(None)
        .build()
        .unwrap();

    let futcheck = dclient.get_catalog(None).unwrap();

    let res = tcore.run(futcheck.collect()).unwrap();
    assert_eq!(res, vec!["r1/i1", "r2"]);

    mockito::reset();
}

#[test]
fn test_catalog_paginate() {
    let repos_p1 = r#"{"repositories": ["r1/i1"]}"#;
    let repos_p2 = r#"{"repositories": ["r2"]}"#;

    mock("GET", "/v2/_catalog?n=1")
        .with_status(200)
        .with_header("Link",
                     &format!(r#"<{}/v2/_catalog?n=21&last=r1/i1>; rel="next""#,
                              mockito::SERVER_URL))
        .with_header("Content-Type", "application/json")
        .with_body(repos_p1)
        .create();
    mock("GET", "/v2/_catalog?n=1&last=r1")
        .with_status(200)
        .with_header("Content-Type", "application/json")
        .with_body(repos_p2)
        .create();

    let mut tcore = Core::new().unwrap();
    let dclient = dkregistry::v2::Client::configure(&tcore.handle())
        .registry(mockito::SERVER_ADDRESS)
        .insecure_registry(true)
        .username(None)
        .password(None)
        .build()
        .unwrap();

    let next = dclient.get_catalog(Some(1)).unwrap();

    let (page1, next) = tcore.run(next.into_future()).ok().unwrap();
    assert_eq!(page1, Some("r1/i1".to_owned()));

    let (page2, next) = tcore.run(next.into_future()).ok().unwrap();
    // TODO(lucab): implement pagination
    assert_eq!(page2, None);

    let (end, _) = tcore.run(next.into_future()).ok().unwrap();
    assert_eq!(end, None);

    mockito::reset();
}
