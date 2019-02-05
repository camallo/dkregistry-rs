//! Error chains, types and traits.

use base64;
use http;
use hyper;
use regex;
use reqwest;
use serde_json;
use std::{io, string};

error_chain! {
    foreign_links {
        Base64Decode(base64::DecodeError);
        HeaderInvalid(hyper::header::InvalidHeaderValue);
        HeaderParse(hyper::header::ToStrError);
        Hyper(hyper::Error);
        Io(io::Error);
        Json(serde_json::Error);
        Regex(regex::Error);
        Reqwest(reqwest::Error);
        UriParse(http::uri::InvalidUri);
        Utf8Parse(string::FromUtf8Error);
    }
}
