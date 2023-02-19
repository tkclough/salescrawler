use std::{fs::{File, self}, io::Write, path::Path};

use serde::{Deserialize, Serialize};
use url::Url;

use crate::{error::Error, auth::make_basic_auth_header, models::Post};

pub struct Client {
    pub config: Config,
    pub auth: Option<Auth>,
}

#[derive(Deserialize, Debug, PartialEq)]
pub struct Config {
    pub auth_host: String,
    pub api_host: String,

    pub token_file: String,

    pub username: String,
    pub password: String,
    pub client_id: String,
    pub client_secret: String,
    pub user_agent: String,

    pub wait_time_secs: u64,
}

#[derive(Deserialize)]
struct AccessTokenResponse {
    access_token: String,
    expires_in: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Auth {
    access_token: String,
    expires_at: std::time::SystemTime,

    ratelimit_used: u64,
    ratelimit_remaining: u64,
    ratelimit_reset: std::time::Duration,
}

impl Client {
    pub const fn new(config: Config) -> Self {
        Self { config, auth: None }
    }

    pub fn is_auth_expired(&self) -> bool {
        let now = std::time::SystemTime::now();
        self.auth
            .as_ref()
            .map_or(true, |auth| auth.expires_at < now)
    }

    pub fn get_wait_time(&self) -> std::time::Duration {
        let wait = std::time::Duration::from_secs(self.config.wait_time_secs);

        self.auth.as_ref().map_or(wait, |auth| {
            if auth.ratelimit_remaining == 0 {
                auth.ratelimit_reset
            } else {
                wait
            }
        })
    }

    pub fn authenticate(&mut self) -> Result<(), Error> {
        let username = urlencoding::encode(&self.config.username);
        let password = urlencoding::encode(&self.config.password);

        let uri = format!("access_token?grant_type=password&username={username}&password={password}");
        let auth_url = Url::parse(&self.config.auth_host)?
            .join(&uri)?;

        let auth_payload = self.get_authorization_header();
        
        let request_time = std::time::SystemTime::now();
        let body = ureq::post(auth_url.as_ref())
            .set("Authorization", &auth_payload)
            .set("User-Agent", &self.config.user_agent)
            .call().map_err(|e| Error::Ureq(Box::new(e)))?
            .into_string()?;
        let AccessTokenResponse { access_token, expires_in } =
            serde_json::from_str(&body)?;
        let expires_at = request_time + std::time::Duration::from_secs(expires_in);

        self.auth = Some(
            Auth {
                access_token,
                expires_at,
                ratelimit_remaining: 1,
                ratelimit_used: 0,
                ratelimit_reset: std::time::Duration::from_secs(3600)
            }
        );

        log::info!("Got new auth: {:?}", self.auth);

        Ok(())
    }

    fn get_authorization_header(&self) -> String {
        make_basic_auth_header(&self.config.client_id, &self.config.client_secret)
    }

    fn add_api_headers(&self, req: ureq::Request) -> Result<ureq::Request, Error> {
        let auth_payload = self.get_auth_payload()?;

        Ok(req.set("Authorization", &auth_payload)
                        .set("User-Agent", &self.config.user_agent))
    }

    fn get_api_url(&self, uri: &str) -> Result<Url, Error> {
        let api_url = Url::parse(&self.config.api_host)?.join(uri)?;
        Ok(api_url)
    }

    fn get(&self, uri: &str) -> Result<ureq::Request, Error> {
        let api_url = self.get_api_url(uri)?;
        let req = self.add_api_headers(ureq::get(api_url.as_ref()))?;
        Ok(req)
    }

    fn get_auth_payload(&self) -> Result<String, Error> {
        self.auth.as_ref().map_or(Err(Error::Reauthenticate), |auth| {
            if self.is_auth_expired() {
                Err(Error::Reauthenticate)
            } else {
                let access_token = &auth.access_token;
                Ok(format!("bearer {access_token}"))
            }
        })
    }

    fn write_auth_to_file(&self) -> Result<(), Error> {
        log::info!("Writing auth to file {}", self.config.token_file);
        let mut file = File::create(&self.config.token_file)?;
        let auth = serde_json::to_string(&self.auth)?;
        file.write_fmt(format_args!("{auth}"))?;
        Ok(())
    }

    pub fn read_auth_from_file(&mut self) -> Result<(), Error> {
        log::info!("Reading auth from file {}", self.config.token_file);
        if !Path::new(&self.config.token_file).exists() {
            log::info!("File doesn't exist");
            return Ok(());
        }
        let contents = fs::read_to_string(&self.config.token_file)?;
        let auth: Auth = serde_json::from_str(&contents)?;
        if auth.expires_at > std::time::SystemTime::now() {
            self.auth = Some(auth);
        }

        log::info!("Successfully read auth from file, {:?}", self.auth);
        Ok(())
    }

    pub fn listing_new(
        &mut self,
        subreddit: &str,
        body: &ListingRequest,
    ) -> Result<ListingResponse, Error> {
        let uri = format!("r/{subreddit}/new");
        let resp = self
            .get(&uri)?
            .send_json(body)
            .map_err(|err| Error::Ureq(Box::new(err)))?;

        self.update_ratelimit_counts(&resp)?;

        let resp = serde_json::from_str(&resp.into_string()?)?;
        Ok(resp)
    }

    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    fn update_ratelimit_counts(&mut self, resp: &ureq::Response) -> Result<(), Error> {
        if let Some(ref mut auth) = self.auth {
            let used: f64 = resp
                .header("X-Ratelimit-Used")
                .ok_or(Error::MissingHeader("X-Ratelimit-Used".to_owned()))?
                .parse()?;

            let remaining: f64 = resp
                .header("X-Ratelimit-Remaining")
                .ok_or(Error::MissingHeader("X-Ratelimit-Remaining".to_owned()))?
                .parse()?;

            let reset: f64 = resp
                .header("X-Ratelimit-Reset")
                .ok_or(Error::MissingHeader("X-Ratelimit-Reset".to_owned()))?
                .parse()?;

            auth.ratelimit_remaining = remaining.floor() as u64;
            auth.ratelimit_used = used.floor() as u64;
            auth.ratelimit_reset = std::time::Duration::from_secs(reset.floor() as u64);

            self.write_auth_to_file()?;
        }

        Ok(())
    }
}

#[derive(Serialize)]
pub struct ListingRequest {
    pub count: u64,
    pub limit: u64,
}

#[derive(Deserialize, Debug)]
pub struct ListingResponse {
    pub data: ListingResponseData,
}

#[derive(Deserialize, Debug)]
pub struct ListingResponseData {
    pub after: Option<String>,
    pub before: Option<String>,
    pub children: Vec<ListingResponseChild>,
}

#[derive(Deserialize, Debug)]
pub struct ListingResponseChild {
    pub data: Post,
}

