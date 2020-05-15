use crate::errors::{Error, Result};
use crate::v2::*;
use bytes::Bytes;
use reqwest;
use reqwest::StatusCode;

impl Client {
    /// Check if a blob exists.
    pub async fn has_blob(&self, name: &str, digest: &str) -> Result<bool> {
        let url = {
            let ep = format!("{}/v2/{}/blobs/{}", self.base_url, name, digest);
            match reqwest::Url::parse(&ep) {
                Ok(url) => url,
                Err(e) => {
                    return Err(Error::from(format!(
                        "failed to parse url from string: {}",
                        e
                    )));
                }
            }
        };

        let res = self
            .build_reqwest(reqwest::Client::new().head(url))
            .send()
            .await?;

        trace!("Blob HEAD status: {:?}", res.status());

        match res.status() {
            StatusCode::OK => Ok(true),
            _ => Ok(false),
        }
    }

    pub async fn get_blob_ref(
        &self,
        name: &str,
        digest: &str,
        bytes_container: &mut Bytes,
    ) -> Result<()> {
        let digest = ContentDigest::try_new(digest.to_string())?;

        let ep = format!("{}/v2/{}/blobs/{}", self.base_url, name, digest);
        let url = reqwest::Url::parse(&ep)
            .map_err(|e| Error::from(format!("failed to parse url from string: {}", e)))?;

        let res = self
            .build_reqwest(reqwest::Client::new().get(url))
            .send()
            .await?;

        trace!("GET {} status: {}", res.url(), res.status());
        let status = res.status();

        if !(status.is_success()
            // Let client errors through to populate them with the body
            || status.is_client_error())
        {
            return Err(Error::from(format!(
                "GET request failed with status '{}'",
                status
            )));
        }

        let status = res.status();
        *bytes_container = res.bytes().await?;
        let len = bytes_container.len();

        if status.is_success() {
            trace!("Successfully received blob with {} bytes ", len);
            digest.try_verify(bytes_container)?;
            return Ok(());
        } else if status.is_client_error() {
            return Err(Error::from(format!(
                "GET request failed with status '{}' and body of size {}: {:#?}",
                status,
                len,
                String::from_utf8_lossy(bytes_container)
            )));
        } else {
            // We only want to handle success and client errors here
            error!(
                "Received unexpected HTTP status '{}' after fetching the body. Please submit a bug report.",
                status
            );
            return Err(Error::from(format!(
                "GET request failed with status '{}'",
                status
            )));
        };
    }

    /// Retrieve blob.
    pub async fn get_blob(&self, name: &str, digest: &str) -> Result<Vec<u8>> {
        let mut body_vec = Bytes::new();
        match self.get_blob_ref(name, digest, &mut body_vec).await {
            Err(e) => Err(e),
            Ok(_) => Ok(body_vec.to_vec()),
        }
    }
}
