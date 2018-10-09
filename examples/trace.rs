extern crate dkregistry;
extern crate env_logger;
extern crate futures;
extern crate log;
extern crate serde_json;
extern crate tokio_core;

use dkregistry::reference;
use std::{boxed, env, error, fs, io};
use tokio_core::reactor::Core;

use std::str::FromStr;

type Result<T> = std::result::Result<T, boxed::Box<error::Error>>;

fn main() {
    let dkr_ref = match std::env::args().nth(1) {
        Some(ref x) => reference::Reference::from_str(x),
        None => reference::Reference::from_str("quay.io/coreos/etcd"),
    }.unwrap();
    let registry = dkr_ref.registry();

    println!("[{}] downloading image {}", registry, dkr_ref);

    let mut user = None;
    let mut password = None;
    let home = env::home_dir().unwrap_or("/root".into());
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

fn run(dkr_ref: &reference::Reference, user: Option<String>, passwd: Option<String>) -> Result<()> {
    env_logger::Builder::new()
        .filter(Some("dkregistry"), log::LevelFilter::Trace)
        .filter(Some("trace"), log::LevelFilter::Trace)
        .try_init()?;
    let image = dkr_ref.repository();
    let version = dkr_ref.version();

    let mut tcore = try!(Core::new());
    let mut dclient = try!(
        dkregistry::v2::Client::configure(&tcore.handle())
            .registry(&dkr_ref.registry())
            .insecure_registry(false)
            .username(user)
            .password(passwd)
            .build()
    );

    let futcheck = try!(dclient.is_v2_supported());
    let supported = try!(tcore.run(futcheck));
    if !supported {
        return Err("API v2 not supported".into());
    }

    let fut_token = try!(dclient.login(vec![&format!("repository:{}:pull", image)]));
    let token_auth = try!(tcore.run(fut_token));

    let futauth = try!(dclient.is_auth(Some(token_auth.token())));
    if !try!(tcore.run(futauth)) {
        return Err("login failed".into());
    }

    dclient.set_token(Some(token_auth.token()));

    let fut_hasmanif = dclient.has_manifest(&image, &version, None)?;
    let manifest_kind = try!(tcore.run(fut_hasmanif)?.ok_or("no manifest found"));

    let fut_manif = dclient.get_manifest(&image, &version)?;
    let body = tcore.run(fut_manif)?;

    let layers = match manifest_kind {
        dkregistry::mediatypes::MediaTypes::ManifestV2S1Signed => {
            let m: dkregistry::v2::manifest::ManifestSchema1Signed =
                try!(serde_json::from_slice(body.as_slice()));
            m.get_layers()
        }
        dkregistry::mediatypes::MediaTypes::ManifestV2S2 => {
            let m: dkregistry::v2::manifest::ManifestSchema2 =
                try!(serde_json::from_slice(body.as_slice()));
            m.get_layers()
        }
        _ => return Err("unknown format".into()),
    };

    for digest in layers {
        let fut_presence = dclient.has_blob(&image, &digest)?;
        let has_blob = tcore.run(fut_presence)?;
        if !has_blob {
            return Err(format!("missing layer {}", digest).into());
        }

        let fut_out = dclient.get_blob(&image, &digest)?;
        let _out = tcore.run(fut_out)?;
    }

    return Ok(());
}
