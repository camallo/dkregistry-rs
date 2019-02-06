extern crate dkregistry;
extern crate spectral;

use dkregistry::reference::Reference;
use spectral::prelude::*;
use std::str::FromStr;

#[test]
fn valid_references() {
    let tcases = vec![
        "library/busybox",
        "busybox",
        "busybox:tag",
        "busybox:5000",
        "busybox@sha256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
        "registry.example.com:5000/library/busybox:5000",
    ];

    for t in tcases {
        let r = Reference::from_str(t);
        asserting(t).that(&r).is_ok();
        let repo = r.unwrap().repository();
        asserting(t)
            .that(&repo.as_str())
            .is_equal_to("library/busybox");
    }
}

#[test]
fn invalid_references() {
    let tcases = vec!["".into(), "L".repeat(128), ":justatag".into()];

    for t in tcases.iter() {
        let r = Reference::from_str(t);
        asserting(t).that(&r).is_err();
    }
}

#[test]
fn hostname_without_namespace() {
    let dkr_ref = Reference::from_str(
        "sat-r220-02.lab.eng.rdu2.redhat.com:5000/default_organization-custom-ocp",
    )
    .unwrap();

    assert_eq!(
        dkr_ref.registry(),
        "sat-r220-02.lab.eng.rdu2.redhat.com:5000"
    );
    assert_eq!(dkr_ref.repository(), "default_organization-custom-ocp");
    assert_eq!(dkr_ref.version(), "latest");
}
