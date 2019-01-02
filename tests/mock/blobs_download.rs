extern crate dkregistry;
extern crate mockito;
extern crate tokio_core;

use self::mockito::mock;
use self::tokio_core::reactor::Core;

#[test]
fn test_blobs_has_layer() {
    let name = "my-repo/my-image";
    let digest = "fakedigest";
    let binary_digest = "binarydigest";

    let ep = format!("/v2/{}/blobs/{}", name, digest);
    let addr = mockito::server_address().to_string();
    let _m = mock("HEAD", ep.as_str())
        .with_status(200)
        .with_header("Content-Length", "0")
        .with_header("Docker-Content-Digest", binary_digest)
        .create();

    let mut tcore = Core::new().unwrap();
    let dclient = dkregistry::v2::Client::configure(&tcore.handle())
        .registry(&addr)
        .insecure_registry(true)
        .username(None)
        .password(None)
        .build()
        .unwrap();

    let futcheck = dclient.has_blob(name, digest);

    let res = tcore.run(futcheck).unwrap();
    assert_eq!(res, true);

    mockito::reset();
}

#[test]
fn test_blobs_hasnot_layer() {
    let name = "my-repo/my-image";
    let digest = "fakedigest";

    let ep = format!("/v2/{}/blobs/{}", name, digest);
    let addr = mockito::server_address().to_string();
    let _m = mock("HEAD", ep.as_str()).with_status(404).create();

    let mut tcore = Core::new().unwrap();
    let dclient = dkregistry::v2::Client::configure(&tcore.handle())
        .registry(&addr)
        .insecure_registry(true)
        .username(None)
        .password(None)
        .build()
        .unwrap();

    let futcheck = dclient.has_blob(name, digest);

    let res = tcore.run(futcheck).unwrap();
    assert_eq!(res, false);

    mockito::reset();
}
