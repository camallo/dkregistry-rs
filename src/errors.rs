//! Error chains, types and traits.

use base64;
use hyper;
use serde_json;
use std::{io, string};

error_chain! {
    foreign_links {
        UriParse(hyper::error::UriError);
        Hyper(hyper::Error);
        Json(serde_json::Error);
        Io(io::Error);
        Utf8Parse(string::FromUtf8Error);
        Base64Decode(base64::DecodeError);
    }
}
