use base64;
use futures::Stream;
use hyper::header;
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
    fn get_token_provider(&self) -> FutureString {
        let url = {
            let ep = format!("{}/v2/", self.base_url);
            match hyper::Uri::from_str(ep.as_str()) {
                Ok(url) => url,
                Err(e) => {
                    return Box::new(futures::future::err::<_, _>(Error::from(format!(
                        "failed to parse url from string: {}",
                        e
                    ))))
                }
            }
        };
        let req = self.new_request(hyper::Method::GET, url);
        let freq = self.hclient.request(req);
        let www_auth = freq
            .from_err()
            .and_then(|r| {
                let a = r
                    .headers()
                    .get(hyper::header::WWW_AUTHENTICATE)
                    .ok_or_else(|| Error::from("get_token: missing Auth header"))?;
                let chal = String::from_utf8(a.as_bytes().to_vec())?;
                Ok(chal)
            }).and_then(move |hdr| {
                let mut auth_ep = "".to_owned();
                let mut service = None;
                for item in hdr.trim_left_matches("Bearer ").split(',') {
                    let kv: Vec<&str> = item.split('=').collect();
                    match (kv.get(0), kv.get(1)) {
                        (Some(&"realm"), Some(v)) => auth_ep = v.trim_matches('"').to_owned(),
                        (Some(&"service"), Some(v)) => service = Some(v.trim_matches('"')),
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
        Box::new(www_auth)
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
    pub fn login(&self, scopes: &[&str]) -> FutureTokenAuth {
        let subclient = self.hclient.clone();
        let creds = self.credentials.clone();
        let scope = scopes
            .iter()
            .fold("".to_string(), |acc, &s| acc + "&scope=" + s);
        let auth = self
            .get_token_provider()
            .and_then(move |token_ep| {
                let auth_ep = token_ep + scope.as_str();
                trace!("Token endpoint: {}", auth_ep);
                hyper::Uri::from_str(auth_ep.as_str()).map_err(|e| e.into())
            }).and_then(move |u| {
                let mut auth_req = hyper::Request::default();
                *auth_req.method_mut() = hyper::Method::GET;
                *auth_req.uri_mut() = u;
                if let Some(c) = creds {
                    let plain = format!("{}:{}", c.0, c.1);
                    let basic = format!("Basic {}", base64::encode(&plain));
                    if let Ok(basic_header) = header::HeaderValue::from_str(&basic) {
                        auth_req
                            .headers_mut()
                            .append(header::AUTHORIZATION, basic_header);
                    } else {
                        // TODO: return an error. seems difficult to match the error type for the whole closure
                        error!("could not parse HeaderValue from '{}'", basic);
                    };
                };
                subclient.request(auth_req).map_err(|e| e.into())
            }).and_then(|r| {
                let status = r.status();
                trace!("Got status {}", status);
                match status {
                    hyper::StatusCode::OK => Ok(r),
                    _ => Err(format!("login: wrong HTTP status '{}'", status).into()),
                }
            }).and_then(|r| {
                r.into_body()
                    .concat2()
                    .map_err(|e| format!("login: failed to fetch the whole body: {}", e).into())
            }).and_then(|body| {
                let s = String::from_utf8(body.into_bytes().to_vec())?;
                serde_json::from_slice(s.as_bytes()).map_err(|e| e.into())
            }).inspect(|_| {
                trace!("Got token");
            });
        Box::new(auth)
    }

    /// Check whether the client is authenticated with the registry.
    pub fn is_auth(&self, token: Option<&str>) -> FutureBool {
        let url = match hyper::Uri::from_str((self.base_url.clone() + "/v2/").as_str()) {
            Ok(url) => url,
            Err(e) => return Box::new(futures::future::err(e.into())),
        };
        let mut req = self.new_request(hyper::Method::GET, url.clone());
        if let Some(t) = token {
            let bearer = format!("Bearer {}", t);
            if let Ok(basic_header) = header::HeaderValue::from_str(&bearer) {
                req.headers_mut()
                    .append(header::AUTHORIZATION, basic_header);
            } else {
                // TODO: return an error. seems difficult to match the error type for the whole closure
                error!("could not parse HeaderValue from '{}'", bearer);
            };
        };

        let freq = self.hclient.request(req);
        let fres = freq
            .from_err()
            .inspect(move |_| {
                trace!("GET {:?}", url);
            }).and_then(move |r| {
                let status = r.status();
                trace!("Got status {}", status);
                match status {
                    hyper::StatusCode::OK => Ok(true),
                    hyper::StatusCode::UNAUTHORIZED => Ok(false),
                    _ => Err(format!("is_auth: wrong HTTP status '{}'", status).into()),
                }
            });
        Box::new(fres)
    }
}
