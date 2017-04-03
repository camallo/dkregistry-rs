use v2::*;

#[derive(Debug,Default,Deserialize,Serialize)]
pub struct Tags {
    name: String,
    tags: Vec<String>,
}
pub type FutureTags = Box<futures::Future<Item = Tags, Error = Error>>;

impl Client {
    /// List existing tags for an image.
    pub fn get_tags(&self, name: &str, limit: Option<u32>) -> Result<FutureTags> {
        let url = {
            let mut s = format!("{}/v2/{}/tags/list", self.base_url, name);
            if let Some(n) = limit {
                s = s + &format!("?n={}", n);
            };
            try!(hyper::Uri::from_str(s.as_str()))
        };
        let req = self.new_request(hyper::Method::Get, url);
        let freq = self.hclient.request(req);
        let fres = freq.map_err(|e| e.into())
            .and_then(move |r| {
                          if r.status() != hyper::status::StatusCode::Ok {
                              return Err(hyper::Error::Status);
                          };
                          Ok(r)
                      })
            .and_then(move |r| {
                          r.body().fold(Vec::new(), |mut v, chunk| {
                    v.extend(&chunk[..]);
                    futures::future::ok::<_, hyper::Error>(v)
                })
                      })
            .and_then(|chunks| {
                          let s = String::from_utf8(chunks).unwrap();
                          Ok(s)
                      })
            .and_then(move |body| {
                          serde_json::from_slice(body.as_bytes()).map_err(|_| hyper::Error::Status)
                      })
            .map_err(|e| e.into());
        return Ok(Box::new(fres));
    }
}
