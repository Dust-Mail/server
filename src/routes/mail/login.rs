use crate::{
    guards::{RateLimiter, User},
    state::Config,
    types::{ErrResponse, Error, ErrorKind, OkResponse, ResponseResult},
    utils::base64_encode,
};

use dust_mail::session::{create_sessions, FullLoginOptions};
use rocket::{serde::json::Json, State};

#[post("/login", data = "<credentials>")]
pub async fn login(
    credentials: Json<FullLoginOptions>,
    user: User,
    _rate_limiter: RateLimiter,
    config: &State<Config>,
) -> ResponseResult<String> {
    if config.mail_proxy().is_none() {
        return Err(ErrResponse::new(
            ErrorKind::BadRequest,
            "This Dust-Mail server does not operate as a mail proxy",
        ));
    }

    let session_token = base64_encode(credentials.0.to_string());

    let auth_config = config.authorization().cloned().unwrap_or_default();

    let connection_limit = auth_config.connection_limit();

    let connection_count = user.mail_sessions().count();

    if &connection_count >= connection_limit {
        return Err(ErrResponse::new(
            ErrorKind::BadRequest,
            "You have reached the limit of concurrent connections",
        ));
    }

    match user.mail_sessions().get(&session_token) {
        Some(_) => Err(ErrResponse::new(
            ErrorKind::BadRequest,
            format!(
                "You already have a session connected to that server with token '{}'",
                session_token
            ),
        )),
        None => {
            let mail_sessions = create_sessions(&credentials)
                .await
                .map_err(|err| ErrResponse::from(Error::from(err)).into())?;

            user.mail_sessions().insert(&session_token, mail_sessions);

            Ok(OkResponse::new(session_token))
        }
    }
}
