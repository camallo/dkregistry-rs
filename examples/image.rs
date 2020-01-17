extern crate dirs;
extern crate futures;
extern crate serde_json;
extern crate tokio;

use dkregistry::render;
use futures::future::try_join_all;
use std::result::Result;
use std::{boxed, env, error, fs, io};

mod common;

#[tokio::main]
async fn main() {
    let registry = match std::env::args().nth(1) {
        Some(x) => x,
        None => "quay.io".into(),
    };

    let image = match std::env::args().nth(2) {
        Some(x) => x,
        None => "coreos/etcd".into(),
    };

    let version = match std::env::args().nth(3) {
        Some(x) => x,
        None => "latest".into(),
    };

    println!("[{}] downloading image {}:{}", registry, image, version);

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

    let res = run(&registry, &image, &version, user, password).await;

    if let Err(e) = res {
        println!("[{}] {}", registry, e);
        std::process::exit(1);
    };
}

async fn run(
    registry: &str,
    image: &str,
    version: &str,
    user: Option<String>,
    passwd: Option<String>,
) -> Result<(), boxed::Box<dyn error::Error>> {
    env_logger::Builder::new()
        .filter(Some("dkregistry"), log::LevelFilter::Trace)
        .filter(Some("trace"), log::LevelFilter::Trace)
        .try_init()?;

    let client = dkregistry::v2::Client::configure()
        .registry(registry)
        .insecure_registry(false)
        .username(user)
        .password(passwd)
        .build()?;

    let login_scope = format!("repository:{}:pull", image);

    let dclient = common::authenticate_client(client, login_scope).await?;
    let manifest = dclient.get_manifest(&image, &version).await?;
    let layers_digests = manifest.layers_digests(None)?;

    println!("{} -> got {} layer(s)", &image, layers_digests.len(),);

    let blob_futures = layers_digests
        .iter()
        .map(|layer_digest| dclient.get_blob(&image, &layer_digest))
        .collect::<Vec<_>>();

    let blobs = try_join_all(blob_futures).await?;

    println!("Downloaded {} layers", blobs.len());

    let path = &format!("{}:{}", &image, &version).replace("/", "_");
    let path = std::path::Path::new(&path);
    if path.exists() {
        return Err(format!("path {:?} already exists, exiting", &path).into());
    }
    // TODO: use async io
    std::fs::create_dir(&path).unwrap();
    let can_path = path.canonicalize().unwrap();

    println!("Unpacking layers to {:?}", &can_path);
    render::unpack(&blobs, &can_path)?;
    Ok(())
}
