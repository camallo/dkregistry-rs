extern crate futures;

use futures::prelude::*;

pub fn authenticate_client(
    mut client: dkregistry::v2::Client,
    login_scope: String,
) -> impl Future<Item = dkregistry::v2::Client, Error = dkregistry::errors::Error> {
    futures::future::ok::<_, dkregistry::errors::Error>(client.clone())
        .and_then(|dclient| {
            dclient.is_v2_supported().and_then(|v2_supported| {
                if !v2_supported {
                    Err("API v2 not supported".into())
                } else {
                    Ok(dclient)
                }
            })
        })
        .and_then(|dclient| {
            dclient.is_auth(None).and_then(move |is_auth| {
                if is_auth {
                    Ok(dclient)
                } else {
                    Err("login required".into())
                }
            })
        })
        .or_else(move |_| {
            client
                .login(&[login_scope.as_str()])
                .and_then(move |token| {
                    client
                        .is_auth(Some(token.token()))
                        .and_then(move |is_auth| {
                            if !is_auth {
                                Err("login failed".into())
                            } else {
                                println!("logged in!");
                                Ok(client.set_token(Some(token.token())).clone())
                            }
                        })
                })
        })
}
