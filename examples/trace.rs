extern crate dirs;
extern crate dkregistry;
extern crate futures;
extern crate serde_json;
extern crate tokio;

mod common;

use dkregistry::reference;
use futures::prelude::*;
use std::str::FromStr;
use std::{boxed, env, error, fs, io};
use tokio::runtime::current_thread::Runtime;

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
) -> Result<(), boxed::Box<dyn error::Error>> {
    env_logger::Builder::new()
        .filter(Some("dkregistry"), log::LevelFilter::Trace)
        .filter(Some("trace"), log::LevelFilter::Trace)
        .try_init()?;

    let image = dkr_ref.repository();
    let version = dkr_ref.version();
    let mut runtime = Runtime::new()?;

    let client = dkregistry::v2::Client::configure()
        .registry(&dkr_ref.registry())
        .insecure_registry(false)
        .username(user)
        .password(passwd)
        .build()?;

    let login_scope = "";

    let futures = common::authenticate_client(client, login_scope.to_string())
        .and_then(|dclient| {
            dclient
                .get_manifest(&image, &version)
                .and_then(|manifest| Ok((dclient, manifest.layers_digests(None)?)))
        })
        .and_then(|(dclient, layers_digests)| {
            let image = image.clone();

            println!("{} -> got {} layer(s)", &image, layers_digests.len(),);

            futures::stream::iter_ok::<_, dkregistry::errors::Error>(layers_digests)
                .and_then(move |layer_digest| {
                    let get_blob_future = dclient.get_blob(&image, &layer_digest);
                    get_blob_future.inspect(move |blob| {
                        println!("Layer {}, got {} bytes.\n", layer_digest, blob.len());
                    })
                })
                .collect()
        });

    let blobs = match runtime.block_on(futures) {
        Ok(blobs) => blobs,
        Err(e) => return Err(Box::new(e)),
    };

    println!("Downloaded {} layers", blobs.len());

    Ok(())
}
