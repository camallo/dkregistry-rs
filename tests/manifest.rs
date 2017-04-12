extern crate dkregistry;
extern crate serde_json;

use std::{io, fs};

#[test]
fn test_deserialize_manifest() {
    let f = fs::File::open("tests/fixtures/quayio_coreos_etcd_latest.json").expect("Missing fixture");
    let bufrd = io::BufReader::new(f);
    let _manif: dkregistry::v2::Manifest = serde_json::from_reader(bufrd).unwrap();
}
