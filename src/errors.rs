use hyper;
use serde_json;
use std::{io, string};
use base64;

error_chain! {
    foreign_links {
        UriParse(hyper::error::UriError);
        Hyper(hyper::Error);
        Json(serde_json::Error);
        Io(io::Error);
        Utf8Parse(string::FromUtf8Error);
        Base64Decode(base64::Base64Error);
    }
}
