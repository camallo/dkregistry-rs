use futures::Stream;
use reqwest;
use reqwest::StatusCode;
use v2::*;

/// Convenience alias for future binary blob.
pub type FutureBlob = Box<futures::Future<Item = Vec<u8>, Error = Error>>;

impl Client {
    /// Check if a blob exists.
    pub fn has_blob(&self, name: &str, digest: &str) -> FutureBool {
        let url = {
            let ep = format!("{}/v2/{}/blobs/{}", self.base_url, name, digest);
            match reqwest::Url::parse(&ep) {
                Ok(url) => url,
                Err(e) => {
                    return Box::new(futures::future::err::<_, _>(Error::from(format!(
                        "failed to parse url from string: {}",
                        e
                    ))));
                }
            }
        };

        let fres = reqwest::async::Client::new()
            .head(url)
            .send()
            .inspect(|res| trace!("Blob HEAD status: {:?}", res.status()))
            .and_then(|res| match res.status() {
                StatusCode::OK => Ok(true),
                _ => Ok(false),
            }).map_err(|e| format!("{}", e).into());
        Box::new(fres)
    }

    /// Retrieve blob.
    pub fn get_blob(&self, name: &str, digest: &str) -> FutureBlob {
        let url = {
            let ep = format!("{}/v2/{}/blobs/{}", self.base_url, name, digest);
            match reqwest::Url::parse(&ep) {
                Ok(url) => url,
                Err(e) => {
                    return Box::new(futures::future::err::<_, _>(Error::from(format!(
                        "failed to parse url from string: {}",
                        e
                    ))));
                }
            }
        };

        let fres = reqwest::async::Client::new()
            .get(url)
            .send()
            .inspect(|res| trace!("Blob GET status: {:?}", res.status()))
            .and_then(|mut res| {
                let body = std::mem::replace(res.body_mut(), reqwest::async::Decoder::empty());
                body.concat2()
            }).map_err(|e| Error::from(format!("{}", e)))
            .and_then(|body| {
                let mut cursor = std::io::Cursor::new(body);
                let mut buf = Vec::new();
                use std::io::Read;
                match cursor.read_to_end(&mut buf) {
                    Ok(size) => {
                        trace!("Received {} bytes from blob", size);
                        Ok(buf)
                    }
                    Err(e) => Err(Error::from(format!("{}", e))),
                }
            });
        Box::new(fres)
    }
}
