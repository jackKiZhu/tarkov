//! An unofficial client library for the [Ecape from Tarkov](https://escapefromtarkov.com) (EFT) API.
//!
//! To get started, login to EFT with `Tarkov::from_email_and_password`, `from_access_token`, or `from_session`.
//! (Additionally, on a new session, a profile must be selected with `select_profile` before continuing.)
//!
//! Once authenticated, the resulting value can be used to make further API requests.
//!
//! See [Tarkov](struct.Tarkov.html) for a list of available methods.
//!
//! For examples, see the `examples` directory in the source tree.

use crate::auth::LoginError;
use crate::profile::ProfileError;
use actix_web::client::Client;
use actix_web::http::StatusCode;
use err_derive::Error;
use flate2::read::ZlibDecoder;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use std::io::Read;

const GAME_VERSION: &str = "0.12.2.5485";
const LAUNCHER_VERSION: &str = "0.9.1.935";
const UNITY_VERSION: &str = "2018.4.13f1";

const LAUNCHER_ENDPOINT: &str = "https://launcher.escapefromtarkov.com";
const PROD_ENDPOINT: &str = "https://prod.escapefromtarkov.com";
const TRADING_ENDPOINT: &str = "https://trading.escapefromtarkov.com";
const RAGFAIR_ENDPOINT: &str = "https://ragfair.escapefromtarkov.com";

mod auth;

/// Structs for the Friend API.
pub mod friend;
/// Helper functions for hardware ID.
pub mod hwid;
/// Structs for the Profile API.
pub mod profile;
/// Structs for the Flea Market (Ragfair) API.
pub mod ragfair;
/// Structs for the Trading API.
pub mod trading;

/// Common error enum returned by most functions.
#[derive(Debug, Error)]
pub enum Error {
    /// A `std::io` error
    #[error(display = "io error: {}", _0)]
    Io(#[error(source)] std::io::Error),
    /// An `actix-web` error sending request.
    #[error(display = "send request error: {}", _0)]
    SendRequestError(#[error(from)] actix_web::client::SendRequestError),
    /// An `actix-web` error parsing response.
    #[error(display = "payload error: {}", _0)]
    PayloadError(#[error(from)] actix_web::client::PayloadError),
    /// A `serde_json` error.
    #[error(display = "json error: {}", _0)]
    Json(#[error(source)] serde_json::error::Error),
    /// Generic non-success response from the API.
    #[error(display = "non-success response from api: {}", _0)]
    Status(StatusCode),

    /// Unidentified error within the EFT API.
    #[error(display = "unidentified login error with error code: {}", _0)]
    UnknownAPIError(u8),
    /// Not authorized to API or profile is not selected.
    #[error(display = "not authorized or game profile not selected")]
    NotAuthorized,
    #[doc(hidden)]
    #[error(display = "login api error: {}", _0)]
    LoginError(#[error(source)] LoginError),
    /// `Profile` API error.
    #[error(display = "profile api error: {}", _0)]
    ProfileError(#[error(source)] ProfileError),
}

/// `Result` alias type.
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Deserialize)]
struct ErrorResponse {
    #[serde(rename = "err")]
    code: u8,
    #[serde(rename = "errmsg")]
    message: Option<String>,
}

/// Client for the EFT API.
pub struct Tarkov {
    client: Client,
    pub hwid: String,
    pub session: String,
}

impl Tarkov {
    /// Login with an email and password.
    pub async fn from_email_and_password(email: &str, password: &str, hwid: &str) -> Result<Self> {
        let client = Client::new();

        let user = auth::login(&client, email, password, &hwid).await?;
        let session = auth::exchange_access_token(&client, &user.access_token, &hwid).await?;

        Ok(Tarkov {
            client,
            hwid: hwid.to_string(),
            session: session.session,
        })
    }

    /// Login with a Bearer token.
    pub async fn from_access_token(access_token: &str, hwid: &str) -> Result<Self> {
        let client = Client::new();
        let session = auth::exchange_access_token(&client, &access_token, &hwid).await?;

        Ok(Tarkov {
            client,
            hwid: hwid.to_string(),
            session: session.session,
        })
    }

    /// Login with a cookie session (AKA `PHPSESSID`).
    pub async fn from_session(session: &str, hwid: &str) -> Result<Self> {
        let client = Client::new();

        Ok(Tarkov {
            client,
            hwid: hwid.to_string(),
            session: session.to_string(),
        })
    }

    async fn post_json<S: serde::Serialize + ?Sized, T: DeserializeOwned>(
        &self,
        url: &str,
        body: &S,
    ) -> Result<T> {
        let mut res = self
            .client
            .post(url)
            .header(
                "User-Agent",
                format!(
                    "UnityPlayer/{} (UnityWebRequest/1.0, libcurl/7.52.0-DEV)",
                    UNITY_VERSION
                ),
            )
            .header("App-Version", format!("EFT Client {}", GAME_VERSION))
            .header("X-Unity-Version", UNITY_VERSION)
            .header("Cookie", format!("PHPSESSID={}", self.session))
            .send_json(&body)
            .await?;

        let body = res.body().await?;
        let mut decode = ZlibDecoder::new(&body[..]);
        let mut body = String::new();
        decode.read_to_string(&mut body)?;

        match res.status() {
            StatusCode::OK => Ok(serde_json::from_slice::<T>(body.as_bytes())?),
            _ => Err(Error::Status(res.status())),
        }
    }

    fn handle_error<T: DeserializeOwned>(&self, error: ErrorResponse, ret: T) -> Result<T> {
        match error.code {
            0 => Ok(ret),
            201 => Err(Error::NotAuthorized)?,
            205 => Err(ProfileError::InvalidUserID)?,
            _ => Err(Error::UnknownAPIError(error.code)),
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
