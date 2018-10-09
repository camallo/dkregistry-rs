use futures::Stream;
use v2::*;

/// Convenience alias for future `TokenAuth` result.
pub type FutureTokenAuth = Box<futures::Future<Item = TokenAuth, Error = Error> + 'static>;

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct TokenAuth {
    token: String,
    expires_in: Option<u32>,
    issued_at: Option<String>,
    refresh_token: Option<String>,
}

impl TokenAuth {
    pub fn token(&self) -> &str {
        self.token.as_str()
    }
}

type FutureString = Box<futures::Future<Item = String, Error = self::Error>>;

impl Client {
    fn get_token_provider(&self) -> Result<FutureString> {
        let url = try!(hyper::Uri::from_str(
            (self.base_url.clone() + "/v2/").as_str()
        ));
        let req = self.new_request(hyper::Method::Get, url);
        let freq = self.hclient.request(req);
        let www_auth = freq
            .and_then(|r| Ok(r))
            .and_then(|r| {
                let a = r
                    .headers()
                    .get_raw("www-authenticate")
                    .ok_or(hyper::Error::Header)?
                    .one()
                    .ok_or(hyper::Error::Header)?;
                let chal = try!(String::from_utf8(a.to_vec()));
                Ok(chal)
            }).from_err::<Error>()
            .and_then(move |hdr| {
                let mut auth_ep = "".to_owned();
                let mut service = None;
                for item in hdr.trim_left_matches("Bearer ").split(',') {
                    let kv: Vec<&str> = item.split('=').collect();
                    match (kv.get(0), kv.get(1)) {
                        (Some(&"realm"), Some(v)) => auth_ep = v.trim_matches('"').to_owned(),
                        (Some(&"service"), Some(v)) => service = Some(v.trim_matches('"').clone()),
                        (Some(&"scope"), _) => {}
                        (_, _) => return Err("unsupported key".to_owned().into()),
                    };
                }
                trace!("Token provider: {}", auth_ep);
                if let Some(sv) = service {
                    auth_ep += &format!("?service={}", sv);
                    trace!("Service identity: {}", sv);
                }
                Ok(auth_ep)
            });
        return Ok(Box::new(www_auth));
    }

    /// Set the token to be used for further registry requests.
    pub fn set_token(&mut self, token: Option<&str>) -> &Self {
        if let Some(ref t) = token {
            self.token = Some(t.to_string());
        }
        self
    }

    /// Perform registry authentication and return an authenticated token.
    ///
    /// On success, the returned token will be valid for all requested scopes.
    pub fn login(&self, scopes: Vec<&str>) -> Result<FutureTokenAuth> {
        let subclient = self.hclient.clone();
        let creds = self.credentials.clone();
        let scope = scopes
            .iter()
            .fold("".to_string(), |acc, &s| acc + "&scope=" + s);
        let auth = self
            .get_token_provider()?
            .and_then(move |token_ep| {
                let auth_ep = token_ep + scope.as_str();
                trace!("Token endpoint: {}", auth_ep);
                hyper::Uri::from_str(auth_ep.as_str()).map_err(|e| e.into())
            }).and_then(move |u| {
                let mut auth_req = client::Request::new(hyper::Method::Get, u);
                if let Some(c) = creds {
                    let hdr = hyper::header::Authorization(hyper::header::Basic {
                        username: c.0,
                        password: Some(c.1),
                    });
                    auth_req.headers_mut().set(hdr);
                };
                subclient.request(auth_req).map_err(|e| e.into())
            }).and_then(|r| {
                trace!("Got status {}", r.status());
                if r.status() != hyper::StatusCode::Ok {
                    Err(Error::from(hyper::Error::Status))
                } else {
                    Ok(r)
                }
            }).and_then(|r| {
                r.body()
                    .fold(Vec::new(), |mut v, chunk| {
                        v.extend(&chunk[..]);
                        futures::future::ok::<_, hyper::Error>(v)
                    }).map_err(|e| e.into())
            }).and_then(|body| {
                let s = String::from_utf8(body)?;
                let ta = serde_json::from_slice(s.as_bytes()).map_err(|e| e.into());
                if let Ok(_) = ta {
                    trace!("Got token");
                };
                ta
            });
        return Ok(Box::new(auth));
    }

    /// Check whether the client is authenticated with the registry.
    pub fn is_auth(&self, token: Option<&str>) -> Result<FutureBool> {
        let url = try!(hyper::Uri::from_str(
            (self.base_url.clone() + "/v2/").as_str()
        ));
        let mut req = self.new_request(hyper::Method::Get, url.clone());
        if let Some(t) = token {
            req.headers_mut()
                .set(hyper::header::Authorization(hyper::header::Bearer {
                    token: t.to_owned(),
                }));
        };

        let freq = self.hclient.request(req);
        let fres = freq
            .map(move |r| {
                trace!("GET {:?}", url);
                r
            }).and_then(move |r| {
                trace!("Got status {}", r.status());
                match r.status() {
                    hyper::StatusCode::Ok => Ok(true),
                    hyper::StatusCode::Unauthorized => Ok(false),
                    _ => Err(hyper::error::Error::Status),
                }
            }).from_err();
        return Ok(Box::new(fres));
    }
}
