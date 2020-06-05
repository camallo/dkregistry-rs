use crate::errors::{Error, Result};
use crate::v2::*;
use reqwest::{StatusCode, Url};

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

impl Client {
    async fn get_token_provider(&self) -> Result<String> {
        let url = {
            let ep = format!("{}/v2/", self.base_url.clone(),);
            match reqwest::Url::parse(&ep) {
                Ok(url) => url,
                Err(e) => {
                    bail!("failed to parse url from string '{}': {}", ep, e);
                }
            }
        };

        let r = self
            .build_reqwest(Method::GET, url.clone())
            .send()
            .map_err(|e| Error::from(format!("{}", e)))
            .await?;

        trace!("GET '{}' status: {:?}", r.url(), r.status());
        let a = r
            .headers()
            .get(reqwest::header::WWW_AUTHENTICATE)
            .ok_or_else(|| Error::from("get_token: missing Auth header"))?;
        let chal = String::from_utf8(a.as_bytes().to_vec())?;

        let (mut auth_ep, service) = parse_hdr_bearer(chal.trim_start_matches("Bearer "))?;

        trace!("Token provider: {}", auth_ep);
        if let Some(sv) = service {
            auth_ep += &format!("?service={}", sv);
            trace!("Service identity: {}", sv);
        }

        Ok(auth_ep)
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
    pub async fn login(&self, scopes: &[&str]) -> Result<TokenAuth> {
        let subclient = self.clone();
        let creds = self.credentials.clone();
        let scope = scopes
            .iter()
            .fold("".to_string(), |acc, &s| acc + "&scope=" + s);

        let token_ep = self.get_token_provider().await?;
        let auth_ep = token_ep + scope.as_str();
        trace!("login: token endpoint: {}", auth_ep);

        let url = reqwest::Url::parse(&auth_ep).map_err(|e| {
            Error::from(format!(
                "failed to parse url from string '{}': {}",
                auth_ep, e
            ))
        })?;

        let auth_req = {
            let auth_req = subclient.build_reqwest(Method::GET, url);
            if let Some(creds) = creds {
                auth_req.basic_auth(creds.0, Some(creds.1))
            } else {
                auth_req
            }
        };

        let r = auth_req.send().await?;
        let status = r.status();
        trace!("login: got status {}", status);
        match status {
            StatusCode::OK => {}
            _ => return Err(format!("login: wrong HTTP status '{}'", status).into()),
        }

        let token_auth = r.json::<TokenAuth>().await?;
        let mut t = token_auth.token().to_string();

        if t == "unauthenticated" {
            bail!("received token with value '{}'", t)
        } else if t.is_empty() {
            bail!("received an empty token")
        };

        // mask the token before logging it
        let chars_count = t.chars().count();
        let mask_start = std::cmp::min(1, chars_count - 1);
        let mask_end = std::cmp::max(chars_count - 1, 1);
        t.replace_range(mask_start..mask_end, &"*".repeat(mask_end - mask_start));

        trace!("login: got token: {:?}", t);

        Ok(token_auth)
    }

    /// Check whether the client is authenticated with the registry.
    pub async fn is_auth(&self, token: Option<&str>) -> Result<bool> {
        let url = {
            let ep = format!("{}/v2/", self.base_url.clone(),);
            match Url::parse(&ep) {
                Ok(url) => url,
                Err(e) => {
                    return Err(Error::from(format!(
                        "failed to parse url from string '{}': {}",
                        ep, e
                    )));
                }
            }
        };

        let req = self.build_reqwest(Method::GET, url.clone());
        let req = if let Some(t) = token {
            req.bearer_auth(t)
        } else {
            debug!("is_auth called without token");
            req
        };

        trace!("Sending request to '{}'", url);

        let resp = req.send().await?;
        trace!("GET '{:?}'", resp);

        let status = resp.status();
        match status {
            reqwest::StatusCode::OK => Ok(true),
            reqwest::StatusCode::UNAUTHORIZED => Ok(false),
            _ => Err(format!("is_auth: wrong HTTP status '{}'", status).into()),
        }
    }

    pub async fn authenticate(mut self, login_scope: String) -> Result<Self> {
        //if !self.is_v2_supported().await? {
        //    return Err("API v2 not supported".into());
        //}

        //if self.is_auth(None).await? {
        //    return Ok(self);
        //}

        let token = self.login(&[login_scope.as_str()]).await?;

        if !self.is_auth(Some(token.token())).await? {
            Err("login failed".into())
        } else {
            trace!("login succeeded");
            self.set_token(Some(token.token()));
            Ok(self)
        }
    }
}

/// This parses a Www-Authenticate header of value Bearer.
///
/// We are only interested in the realm and service keys.
fn parse_hdr_bearer(input: &str) -> Result<(String, Option<&str>)> {
    let mut auth_ep = "".to_string();
    let mut service = None;

    let re = regex::Regex::new(r#"(([a-z]+)="([^"]*)")"#)?;
    for capture in re.captures_iter(input) {
        // The indices for this capture are as follows:
        // 0: full match
        // 1: outer group match
        // 2: first nested group match
        // 3: second nested group match
        // Hence, we are interested in the sub-group matches i.e. in 2 and 3.
        let key = capture.get(2).map(|m| m.as_str());
        let value = capture.get(3).map(|m| m.as_str());

        match (key, value) {
            (Some("realm"), Some(v)) => auth_ep = v.trim_matches('"').to_owned(),
            (Some("service"), Some(v)) => service = Some(v.trim_matches('"')),
            (Some("scope"), _) => {}
            (key, _) => return Err(format!("unsupported key '{:?}'", key).into()),
        };
    }

    Ok((auth_ep, service))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bearer_realm_parses_correctly() -> Result<()> {
        let realm = "https://sat-r220-02.lab.eng.rdu2.redhat.com/v2/token";
        let service = "sat-r220-02.lab.eng.rdu2.redhat.com";
        let scope = "repository:registry:pull,push";

        let www_auth_header = format!(
            r#"Bearer realm="{}",service="{}",scope=""{}"#,
            realm, service, scope
        );
        let trimmed_header = www_auth_header.trim_start_matches("Bearer ");

        assert_eq!(
            parse_hdr_bearer(trimmed_header)?,
            (String::from(realm), Some(service))
        );

        Ok(())
    }
}
