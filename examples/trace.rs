extern crate dirs;
extern crate futures;
extern crate serde_json;
extern crate tokio;

mod common;

use dkregistry::reference;
use std::str::FromStr;
use std::{boxed, env, error, fs, io};

#[tokio::main]
async fn main() {
    let dkr_ref = match std::env::args().nth(1) {
        Some(ref x) => reference::Reference::from_str(x),
        None => reference::Reference::from_str("quay.io/coreos/etcd"),
    }
    .unwrap();
    let registry = dkr_ref.registry();

    println!("[{registry}] downloading image {dkr_ref}");

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
            println!("[{registry}] no credentials found in config.json");
        }
    } else {
        user = env::var("DKREG_USER").ok();
        if user.is_none() {
            println!("[{registry}] no $DKREG_USER for login user");
        }
        password = env::var("DKREG_PASSWD").ok();
        if password.is_none() {
            println!("[{registry}] no $DKREG_PASSWD for login password");
        }
    };

    let res = run(&dkr_ref, user, password).await;

    if let Err(e) = res {
        println!("[{registry}] {e:?}");
        std::process::exit(1);
    };
}

async fn run(
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

    let client = dkregistry::v2::Client::configure()
        .registry(&dkr_ref.registry())
        .insecure_registry(false)
        .username(user)
        .password(passwd)
        .build()?;

    let login_scope = "";

    let dclient = client.authenticate(&[login_scope]).await?;
    let manifest = dclient.get_manifest(&image, &version).await?;

    let layers_digests = manifest.layers_digests(None)?;
    println!("{} -> got {} layer(s)", &image, layers_digests.len(),);

    for layer_digest in &layers_digests {
        let blob = dclient.get_blob(&image, layer_digest).await?;
        println!("Layer {}, got {} bytes.\n", layer_digest, blob.len());
    }

    println!("Downloaded {} layers", layers_digests.len());

    Ok(())
}
