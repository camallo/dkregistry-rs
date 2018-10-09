extern crate dkregistry;
extern crate spectral;

use dkregistry::reference::Reference;
use spectral::prelude::*;
use std::str::FromStr;

#[test]
fn test_reference_repo() {
    let tcases = vec![
        "library/busybox",
        "busybox",
        "busybox:tag",
        "busybox:5000",
        "busybox@sha256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
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
fn test_reference_error() {
    let tcases = vec!["".into(), "L".repeat(128), ":justatag".into()];

    for t in tcases.iter() {
        let r = Reference::from_str(t);
        asserting(t).that(&r).is_err();
    }
}
