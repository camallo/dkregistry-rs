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

        let fres = self
            .build_reqwest(reqwest::async::Client::new().head(url))
            .send()
            .inspect(|res| trace!("Blob HEAD status: {:?}", res.status()))
            .and_then(|res| match res.status() {
                StatusCode::OK => Ok(true),
                _ => Ok(false),
            })
            .map_err(|e| format!("{}", e).into());
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

        let fres = self.build_reqwest(reqwest::async::Client::new().get(url))
            .send()
            .map_err(|e| ::errors::Error::from(format!("{}", e)))
            .and_then(|res| {
                trace!("Blob GET status: {:?}", res.status());
                let status = res.status();

                if status.is_success()
                    // Let client errors through to populate them with the body
                    || status.is_client_error()
                {
                    Ok(res)
                } else {
                    Err(::errors::Error::from(format!(
                        "GET request failed with status '{}'",
                        status
                    )))
                }
            }).and_then(|mut res| {
                std::mem::replace(res.body_mut(), reqwest::async::Decoder::empty())
                    .concat2()
                    .map_err(|e| ::errors::Error::from(format!("{}", e)))
                    .join(futures::future::ok(res))
            }).map_err(|e| ::errors::Error::from(format!("{}", e)))
            .and_then(|(body, res)| {
                let body_vec = body.to_vec();
                let len = body_vec.len();
                let status = res.status();

                if status.is_success() {
                    trace!("Successfully received blob with {} bytes ", len);
                    Ok(body_vec)
                } else if status.is_client_error() {
                    Err(Error::from(format!(
                        "GET request failed with status '{}' and body of size {}: {:#?}",
                        status,
                        len,
                        String::from_utf8_lossy(&body_vec)
                    )))
                } else {
                    // We only want to handle success and client errors here
                    error!(
                        "Received unexpected HTTP status '{}' after fetching the body. Please submit a bug report.",
                        status
                    );
                    Err(Error::from(format!(
                        "GET request failed with status '{}'",
                        status
                    )))
                }
            });
        Box::new(fres)
    }
}
