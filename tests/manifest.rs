extern crate dkregistry;
extern crate serde_json;

use std::collections::HashMap;
use std::{fs, io};

#[test]
fn test_deserialize_manifest_v2s1_signed() {
    let f = fs::File::open("tests/fixtures/manifest_v2_s1.json").expect("Missing fixture");
    let bufrd = io::BufReader::new(f);
    let _manif: dkregistry::v2::manifest::ManifestSchema1Signed =
        serde_json::from_reader(bufrd).unwrap();
}

#[test]
fn test_deserialize_manifest_v2s2() {
    let f = fs::File::open("tests/fixtures/manifest_v2_s2.json").expect("Missing fixture");
    let bufrd = io::BufReader::new(f);
    let _manif: dkregistry::v2::manifest::ManifestSchema2 = serde_json::from_reader(bufrd).unwrap();
}

#[test]
fn test_deserialize_manifest_list_v2() {
    let f = fs::File::open("tests/fixtures/manifest_list_v2.json").expect("Missing fixture");
    let bufrd = io::BufReader::new(f);
    let _manif: dkregistry::v2::manifest::ManifestList = serde_json::from_reader(bufrd).unwrap();
}

#[test]
fn test_deserialize_etcd_manifest() {
    let f =
        fs::File::open("tests/fixtures/quayio_coreos_etcd_latest.json").expect("Missing fixture");
    let bufrd = io::BufReader::new(f);
    let _manif: dkregistry::v2::manifest::ManifestSchema1Signed =
        serde_json::from_reader(bufrd).unwrap();
}

#[test]
fn test_labels_manifest_v2s1_signed() {
    let f = fs::File::open("tests/fixtures/manifest_v2_s1.json").expect("Missing fixture");
    let bufrd = io::BufReader::new(f);
    let manif: dkregistry::v2::manifest::ManifestSchema1Signed =
        serde_json::from_reader(bufrd).unwrap();
    assert_eq!(None, manif.get_labels(0));

    let f =
        fs::File::open("tests/fixtures/quayio_steveej_cincinnati-test-labels_dkregistry-test.json")
            .expect("Missing fixture");
    let bufrd = io::BufReader::new(f);
    let manif: dkregistry::v2::manifest::ManifestSchema1Signed =
        serde_json::from_reader(bufrd).unwrap();

    let labels_0 = manif.get_labels(0).expect("Missing labels");
    let mut expected_labels_0: HashMap<String, String> = HashMap::new();
    expected_labels_0.insert("channel".into(), "beta".into());
    assert_eq!(expected_labels_0, labels_0);
    assert_eq!(None, manif.get_labels(1));
}
