extern crate dkregistry;
extern crate futures;
extern crate serde_json;
extern crate tokio_core;

use self::dkregistry::mediatypes::MediaTypes;
use self::dkregistry::v2::manifest::ManifestSchema1Signed;
use self::futures::future;
use self::futures::prelude::*;
use self::tokio_core::reactor::Core;

static REGISTRY: &'static str = "quay.io";

fn get_env() -> Option<(String, String)> {
    let user = ::std::env::var("DKREG_QUAY_USER");
    let password = ::std::env::var("DKREG_QUAY_PASSWD");
    match (user, password) {
        (Ok(u), Ok(t)) => Some((u, t)),
        _ => None,
    }
}

#[test]
fn test_quayio_getenv() {
    if get_env().is_none() {
        println!(
            "[WARN] {}: missing DKREG_QUAY_USER / DKREG_QUAY_PASSWD",
            REGISTRY
        );
    }
}

#[test]
fn test_quayio_base() {
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

    let futcheck = dclient.is_v2_supported();

    let res = tcore.run(futcheck).unwrap();
    assert_eq!(res, true);
}

#[test]
fn test_quayio_insecure() {
    let mut tcore = Core::new().unwrap();
    let dclient = dkregistry::v2::Client::configure(&tcore.handle())
        .registry(REGISTRY)
        .insecure_registry(true)
        .username(None)
        .password(None)
        .build()
        .unwrap();

    let futcheck = dclient.is_v2_supported();

    let res = tcore.run(futcheck).unwrap();
    assert_eq!(res, false);
}

#[test]
fn test_quayio_auth_login() {
    let login_scope = "";
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

    let futlogin = futures::future::ok(dclient).and_then(|dclient| {
        dclient
            .login(&[&login_scope])
            .and_then(move |token| dclient.is_auth(Some(token.token())))
    });

    let res = tcore.run(futlogin).unwrap();
    assert_eq!(res, true);
}

#[test]
fn test_quayio_get_tags_simple() {
    let mut tcore = Core::new().unwrap();
    let dclient = dkregistry::v2::Client::configure(&tcore.handle())
        .registry(REGISTRY)
        .insecure_registry(false)
        .username(None)
        .password(None)
        .build()
        .unwrap();

    let image = "coreos/alpine-sh";
    let fut_tags = dclient.get_tags(image, None);
    let tags = tcore.run(fut_tags.collect()).unwrap();
    let has_version = tags.iter().any(|t| t == "latest");

    assert_eq!(has_version, true);
}

#[test]
fn test_quayio_get_tags_limit() {
    let mut tcore = Core::new().unwrap();
    let dclient = dkregistry::v2::Client::configure(&tcore.handle())
        .registry(REGISTRY)
        .insecure_registry(false)
        .username(None)
        .password(None)
        .build()
        .unwrap();

    let image = "coreos/alpine-sh";
    let fut_tags = dclient.get_tags(image, Some(10));
    let tags = tcore.run(fut_tags.collect()).unwrap();
    let has_version = tags.iter().any(|t| t == "latest");

    assert_eq!(has_version, true);
}

#[test]
fn test_quayio_get_tags_pagination() {
    let mut tcore = Core::new().unwrap();
    let dclient = dkregistry::v2::Client::configure(&tcore.handle())
        .registry(REGISTRY)
        .insecure_registry(false)
        .username(None)
        .password(None)
        .build()
        .unwrap();

    let image = "coreos/flannel";
    let fut_tags = dclient.get_tags(image, Some(20));
    let tags = tcore.run(fut_tags.collect()).unwrap();
    let has_version = tags.iter().any(|t| t == "v0.10.0");

    assert_eq!(has_version, true);
}

#[test]
fn test_quayio_auth_tags() {
    let image = "steveej/cincinnati-test";
    let login_scope = format!("repository:{}:pull", image);
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

    let fut_tags = future::ok(dclient).and_then(|dclient| {
        dclient
            .login(&[&login_scope])
            .and_then(move |auth| {
                let token = auth.token().to_string();
                dclient
                    .is_auth(Some(&token))
                    .map(move |ok| (dclient, token, ok))
            }).and_then(|(mut dclient, token, ok)| {
                ensure!(ok, "authentication failed");
                dclient.set_token(Some(&token));
                Ok(dclient)
            }).and_then(|dclient| dclient.get_tags(image, None).collect())
    });

    let tags = tcore.run(fut_tags).unwrap();
    let has_version = tags.iter().any(|t| t == "0.0.1");
    assert_eq!(has_version, true);
}

#[test]
fn test_quayio_has_manifest() {
    let mut tcore = Core::new().unwrap();
    let dclient = dkregistry::v2::Client::configure(&tcore.handle())
        .registry(REGISTRY)
        .insecure_registry(false)
        .username(None)
        .password(None)
        .build()
        .unwrap();

    let image = "coreos/alpine-sh";
    let reference = "latest";
    let fut = dclient.has_manifest(image, reference, None);
    let has_manifest = tcore.run(fut).unwrap();

    assert_eq!(has_manifest, Some(MediaTypes::ManifestV2S1Signed));
}

#[test]
fn test_quayio_auth_manifest() {
    let image = "steveej/cincinnati-test";
    let reference = "0.0.1";
    let login_scope = format!("repository:{}:pull", image);
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

    let fut_has_manifest = future::ok(dclient).and_then(|dclient| {
        dclient
            .login(&[&login_scope])
            .and_then(move |auth| {
                let token = auth.token().to_string();
                dclient
                    .is_auth(Some(&token))
                    .map(move |ok| (dclient, token, ok))
            }).and_then(|(mut dclient, token, ok)| {
                ensure!(ok, "authentication failed");
                dclient.set_token(Some(&token));
                Ok(dclient)
            }).and_then(|dclient| dclient.has_manifest(image, reference, None))
    });

    let has_manifest = tcore.run(fut_has_manifest).unwrap();
    assert_eq!(has_manifest, Some(MediaTypes::ManifestV2S1Signed));
}

#[test]
fn test_quayio_has_no_manifest() {
    let mut tcore = Core::new().unwrap();
    let dclient = dkregistry::v2::Client::configure(&tcore.handle())
        .registry(REGISTRY)
        .insecure_registry(false)
        .username(None)
        .password(None)
        .build()
        .unwrap();

    let image = "coreos/alpine-sh";
    let reference = "clearly_bogus";
    let fut = dclient.has_manifest(image, reference, None);
    let has_manifest = tcore.run(fut).unwrap();

    assert_eq!(has_manifest, None);
}

#[test]
fn test_quayio_auth_layer_blob() {
    let image = "steveej/cincinnati-test";
    let reference = "0.0.1";
    let layer0_sha = "sha256:dc9b1c7fec43c5c9655c00e0042847320faadf2a86379cbb8df1eaafade971e5";
    let layer0_len: usize = 199;

    let login_scope = format!("repository:{}:pull", image);
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

    let fut_layer0_blob = future::ok(dclient).and_then(|dclient| {
        dclient
            .login(&[&login_scope])
            .and_then(move |auth| {
                let token = auth.token().to_string();
                dclient
                    .is_auth(Some(&token))
                    .map(move |ok| (dclient, token, ok))
            }).and_then(|(mut dclient, token, ok)| {
                ensure!(ok, "authentication failed");
                dclient.set_token(Some(&token));
                Ok(dclient)
            }).and_then(|dclient| {
                dclient
                    .get_manifest(image, reference)
                    .map(|manifest| (dclient, manifest))
            }).and_then(|(dclient, manifest)| {
                let m: ManifestSchema1Signed = serde_json::from_slice(manifest.as_slice()).unwrap();
                let layers = m.get_layers();
                let num_layers = layers.len();
                ensure!(num_layers == 1, "layers length: {}", num_layers);
                let digest = layers[0].clone();
                ensure!(digest == layer0_sha, "layer0 digest: {}", digest);
                Ok((dclient, digest))
            }).and_then(|(dclient, digest)| dclient.get_blob(&image, &digest))
    });

    let layer0_blob = tcore.run(fut_layer0_blob).unwrap();
    assert_eq!(layer0_blob.len(), layer0_len);
}
