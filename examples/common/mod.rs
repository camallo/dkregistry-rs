extern crate dkregistry;
extern crate futures;

use futures::prelude::*;

pub fn authenticate_client<'a>(
    client: &'a mut dkregistry::v2::Client,
    login_scope: &'a str,
) -> impl futures::future::Future<Item = &'a dkregistry::v2::Client, Error = dkregistry::errors::Error>
{
    futures::future::ok::<_, dkregistry::errors::Error>(client)
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
            dclient.is_auth(None).and_then(|is_auth| {
                if is_auth {
                    Err("no login performed, but already authenticated".into())
                } else {
                    Ok(dclient)
                }
            })
        })
        .and_then(move |dclient| {
            dclient.login(&[&login_scope]).and_then(move |token| {
                dclient
                    .is_auth(Some(token.token()))
                    .and_then(move |is_auth| {
                        if !is_auth {
                            Err("login failed".into())
                        } else {
                            println!("logged in!");
                            Ok(dclient.set_token(Some(token.token())))
                        }
                    })
            })
        })
}
