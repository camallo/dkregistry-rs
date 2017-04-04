extern crate dkregistry;
extern crate mockito;
extern crate tokio_core;
extern crate futures;

use self::mockito::mock;
use self::tokio_core::reactor::Core;
use self::futures::Stream;

#[test]
fn test_tags_simple() {
    let name = "repo";
    let tags = r#"{"name": "repo", "tags": [ "t1", "t2" ]}"#;

    let ep = format!("/v2/{}/tags/list", name);
    mock("GET", ep.as_str())
        .with_status(200)
        .with_header("Content-Type", "application/json")
        .with_body(tags)
        .create();

    let mut tcore = Core::new().unwrap();
    let dclient = dkregistry::v2::Client::configure(&tcore.handle())
        .registry(mockito::SERVER_ADDRESS)
        .insecure_registry(true)
        .username(None)
        .password(None)
        .build()
        .unwrap();

    let futcheck = dclient.get_tags(name, None).unwrap();

    let res = tcore.run(futcheck.collect()).unwrap();
    assert_eq!(res, vec!["t1", "t2"]);

    mockito::reset();
}

#[test]
fn test_tags_paginate() {
    let name = "repo";
    let tags_p1 = r#"{"name": "repo", "tags": [ "t1" ]}"#;
    let tags_p2 = r#"{"name": "repo", "tags": [ "t2" ]}"#;

    let ep1 = format!("/v2/{}/tags/list?n=1", name);
    let ep2 = format!("/v2/{}/tags/list?n=1&last=t1", name);
    mock("GET", &ep1)
        .with_status(200)
        .with_header("Link",
                     &format!(r#"<{}/v2/_tags?n=1&last=t1>; rel="next""#,
                              mockito::SERVER_URL))
        .with_header("Content-Type", "application/json")
        .with_body(tags_p1)
        .create();
    mock("GET", &ep2)
        .with_status(200)
        .with_header("Content-Type", "application/json")
        .with_body(tags_p2)
        .create();

    let mut tcore = Core::new().unwrap();
    let dclient = dkregistry::v2::Client::configure(&tcore.handle())
        .registry(mockito::SERVER_ADDRESS)
        .insecure_registry(true)
        .username(None)
        .password(None)
        .build()
        .unwrap();

    let next = dclient.get_tags(name, Some(1)).unwrap();

    let (page1, next) = tcore.run(next.into_future()).ok().unwrap();
    assert_eq!(page1, Some("t1".to_owned()));

    let (page2, next) = tcore.run(next.into_future()).ok().unwrap();
    // TODO(lucab): implement pagination
    assert_eq!(page2, None);

    let (end, _) = tcore.run(next.into_future()).ok().unwrap();
    assert_eq!(end, None);

    mockito::reset();
}
