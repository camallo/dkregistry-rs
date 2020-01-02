extern crate futures;

pub async fn authenticate_client(
    mut client: dkregistry::v2::Client,
    login_scope: String,
) -> Result<dkregistry::v2::Client, dkregistry::errors::Error> {
    if !client.is_v2_supported().await? {
        return Err("API v2 not supported".into());
    }

    if client.is_auth(None).await? {
        return Ok(client);
    }

    let token = client.login(&[login_scope.as_str()]).await?;

    if !client.is_auth(Some(token.token())).await? {
        Err("login failed".into())
    } else {
        println!("logged in!");
        Ok(client.set_token(Some(token.token())).clone())
    }
}
