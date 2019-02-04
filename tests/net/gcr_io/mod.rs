extern crate dkregistry;
extern crate futures;
extern crate tokio;

use self::futures::stream::Stream;
use self::tokio::runtime::current_thread::Runtime;

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
        println!(
            "[WARN] {}: missing DKREG_GCR_USER / DKREG_GCR_PASSWD",
            REGISTRY
        );
    }
}

#[test]
fn test_gcrio_base() {
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
fn test_gcrio_insecure() {
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

#[test]
fn test_gcrio_get_tags() {
    let mut runtime = Runtime::new().unwrap();
    let dclient = dkregistry::v2::Client::configure()
        .registry(REGISTRY)
        .insecure_registry(false)
        .username(None)
        .password(None)
        .build()
        .unwrap();

    let image = "google_containers/mounttest";
    let fut_tags = dclient.get_tags(image, None);
    let tags = runtime.block_on(fut_tags.collect()).unwrap();
    let has_version = tags.iter().any(|t| t == "0.2");

    assert_eq!(has_version, true);
}

#[test]
fn test_gcrio_has_manifest() {
    let mut runtime = Runtime::new().unwrap();
    let dclient = dkregistry::v2::Client::configure()
        .registry(REGISTRY)
        .insecure_registry(false)
        .username(None)
        .password(None)
        .build()
        .unwrap();

    let image = "google_containers/mounttest";
    let tag = "0.2";
    let manifest_type = dkregistry::mediatypes::MediaTypes::ManifestV2S1Signed.to_string();
    let fut = dclient.has_manifest(image, tag, Some(vec![manifest_type.as_str()].as_slice()));
    let has_manifest = runtime.block_on(fut).unwrap();

    assert_eq!(
        has_manifest,
        Some(dkregistry::mediatypes::MediaTypes::ManifestV2S1Signed)
    );
}
