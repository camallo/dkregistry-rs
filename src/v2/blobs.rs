use crate::errors::{Error, Result};
use crate::v2::*;

use std::pin::Pin;

use bytes::Bytes;
use futures::stream::Stream;
use futures::task::{Context, Poll};
use pin_project::pin_project;
use reqwest::{self, Method, StatusCode};

impl Client {
    /// Check if a blob exists.
    pub async fn has_blob(&self, name: &str, digest: &str) -> Result<bool> {
        let url = {
            let ep = format!("{}/v2/{}/blobs/{}", self.base_url, name, digest);
            reqwest::Url::parse(&ep)?
        };

        let res = self.build_reqwest(Method::HEAD, url.clone()).send().await?;

        trace!("Blob HEAD status: {:?}", res.status());

        match res.status() {
            StatusCode::OK => Ok(true),
            _ => Ok(false),
        }
    }

    async fn get_blob_response(&self, name: &str, digest: &str) -> Result<reqwest::Response> {
        let ep = format!("{}/v2/{}/blobs/{}", self.base_url, name, digest);
        let url = reqwest::Url::parse(&ep)?;

        let res = self.build_reqwest(Method::GET, url.clone()).send().await?;

        let status = res.status();
        trace!("GET {} status: {}", res.url(), status);

        match res.error_for_status_ref() {
            Ok(_) => {
                if let Some(len) = res.content_length() {
                    trace!("Receiving a blob with {} bytes", len);
                } else {
                    trace!("Receiving a blob");
                }
                Ok(res)
            }
            Err(_) if status.is_client_error() => Err(Error::Client { status }),
            Err(_) if status.is_server_error() => Err(Error::Server { status }),
            Err(_) => {
                error!("Received unexpected HTTP status '{}'", status);
                Err(Error::UnexpectedHttpStatus(status))
            }
        }
    }

    /// Retrieve blob.
    pub async fn get_blob(&self, name: &str, digest: &str) -> Result<Vec<u8>> {
        let blob_resp = self.get_blob_response(name, digest).await?;
        let blob = blob_resp.bytes().await?.to_vec();

        let mut digest = ContentDigest::try_new(digest)?;
        digest.update(&blob);
        digest.verify()?;

        Ok(blob.to_vec())
    }

    /// Retrieve blob stream.
    pub async fn get_blob_stream(
        &self,
        name: &str,
        digest: &str,
    ) -> Result<impl Stream<Item = Result<Vec<u8>>>> {
        let blob_resp = self.get_blob_response(name, digest).await?;
        let blob_stream = blob_resp.bytes_stream();

        Ok(BlobStream {
            stream: blob_stream,
            digest: Some(ContentDigest::try_new(digest)?),
        })
    }
}

#[pin_project]
struct BlobStream<S>
where
    S: Stream<Item = reqwest::Result<Bytes>>,
{
    #[pin]
    stream: S,
    #[pin]
    digest: Option<ContentDigest>,
}

impl<S> Stream for BlobStream<S>
where
    S: Stream<Item = reqwest::Result<Bytes>> + Unpin,
{
    type Item = Result<Vec<u8>>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();
        match this.stream.poll_next(cx) {
            Poll::Ready(Some(chunk_res)) => {
                let mut digest = match this.digest.as_pin_mut() {
                    Some(digest) => digest,
                    None => return Poll::Ready(None),
                };
                let chunk = chunk_res?;
                digest.update(&chunk);
                Poll::Ready(Some(Ok(chunk.to_vec())))
            }
            Poll::Ready(None) => match this.digest.take() {
                Some(digest) => match digest.verify() {
                    Ok(()) => Poll::Ready(None),
                    Err(err) => Poll::Ready(Some(Err(err.into()))),
                },
                None => Poll::Ready(None),
            },
            Poll::Pending => Poll::Pending,
        }
    }
}
