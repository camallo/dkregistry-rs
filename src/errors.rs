extern crate hyper;
extern crate serde_json;

error_chain! {
    foreign_links {
        UriParse(hyper::error::UriError);
        Hyper(hyper::Error);
        Json(serde_json::Error);
        Io(::std::io::Error);
        Utf8Parse(::std::string::FromUtf8Error);
    }
}
