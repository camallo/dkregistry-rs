extern crate dirs;
extern crate dkregistry;
extern crate env_logger;
extern crate futures;
extern crate log;
extern crate serde_json;
extern crate tokio_core;

mod common;

use dkregistry::reference;
use futures::prelude::*;
use std::str::FromStr;
use std::{boxed, env, error, fs, io};
use tokio_core::reactor::Core;

fn main() {
    let dkr_ref = match std::env::args().nth(1) {
        Some(ref x) => reference::Reference::from_str(x),
        None => reference::Reference::from_str("quay.io/coreos/etcd"),
    }
    .unwrap();
    let registry = dkr_ref.registry();

    println!("[{}] downloading image {}", registry, dkr_ref);

    let mut user = None;
    let mut password = None;
    let home = dirs::home_dir().unwrap();
    let cfg = fs::File::open(home.join(".docker/config.json"));
    if let Ok(fp) = cfg {
        let creds = dkregistry::get_credentials(io::BufReader::new(fp), &registry);
        if let Ok(user_pass) = creds {
            user = user_pass.0;
            password = user_pass.1;
        } else {
            println!("[{}] no credentials found in config.json", registry);
        }
    } else {
        user = env::var("DKREG_USER").ok();
        if user.is_none() {
            println!("[{}] no $DKREG_USER for login user", registry);
        }
        password = env::var("DKREG_PASSWD").ok();
        if password.is_none() {
            println!("[{}] no $DKREG_PASSWD for login password", registry);
        }
    };

    let res = run(&dkr_ref, user, password);

    if let Err(e) = res {
        println!("[{}] {:?}", registry, e);
        std::process::exit(1);
    };
}

fn run(
    dkr_ref: &reference::Reference,
    user: Option<String>,
    passwd: Option<String>,
) -> Result<(), boxed::Box<error::Error>> {
    env_logger::Builder::new()
        .filter(Some("dkregistry"), log::LevelFilter::Trace)
        .filter(Some("trace"), log::LevelFilter::Trace)
        .try_init()?;

    let image = dkr_ref.repository();
    let version = dkr_ref.version();
    let mut tcore = Core::new()?;

    let mut client = dkregistry::v2::Client::configure(&tcore.handle())
        .registry(&dkr_ref.registry())
        .insecure_registry(false)
        .username(user)
        .password(passwd)
        .build()?;

    let login_scope = "";

    let futures = common::authenticate_client(&mut client, &login_scope)
        .and_then(|dclient| {
            dclient
                    .has_manifest(&image, &version, None)
                    .and_then(move |manifest_option| Ok((dclient, manifest_option)))
                    .and_then(|(dclient, manifest_option)| match manifest_option {
                        None => {
                            return Err(
                                format!("{}:{} doesn't have a manifest", &image, &version).into()
                            )
                        }

                        Some(manifest_kind) => Ok((dclient, manifest_kind)),
                    })
        })
        .and_then(|(dclient, manifest_kind)| {
            let image = image.clone();
            dclient
                .get_manifest(&image, &version)
                .and_then(move |manifest_body| {
                    let layers = match manifest_kind {
                        dkregistry::mediatypes::MediaTypes::ManifestV2S1Signed => {
                            let m: dkregistry::v2::manifest::ManifestSchema1Signed =
                                serde_json::from_slice(manifest_body.as_slice()).unwrap();
                            m.get_layers()
                        }
                        dkregistry::mediatypes::MediaTypes::ManifestV2S2 => {
                            let m: dkregistry::v2::manifest::ManifestSchema2 =
                                serde_json::from_slice(manifest_body.as_slice()).unwrap();
                            m.get_layers()
                        }
                        _ => return Err("unknown format".into()),
                    };
                    Ok((dclient, layers))
                })
        })
        .and_then(|(dclient, layers)| {
            let image = image.clone();

            println!("{} -> got {} layer(s)", &image, layers.len(),);

            futures::stream::iter_ok::<_, dkregistry::errors::Error>(layers)
                .and_then(move |layer| {
                    let get_blob_future = dclient.get_blob(&image, &layer);
                    get_blob_future.inspect(move |blob| {
                        println!("Layer {}, got {} bytes.\n", layer, blob.len());
                    })
                })
                .collect()
        });

    let blobs = match tcore.run(futures) {
        Ok(blobs) => blobs,
        Err(e) => return Err(Box::new(e)),
    };

    println!("Downloaded {} layers", blobs.len());

    Ok(())
}
