use serde::{Deserialize, Serialize};
use ureq::{Request, Response};

use crate::error::Error;

#[derive(Deserialize, PartialEq, Debug)]
pub struct Config {
    pub token: String,
    pub user_agent: String,
    pub api_url: String,
    pub channel_id: String,
    pub sending_interval_secs: u64,
}

pub struct Client {
    config: Config,
    ratelimit: Ratelimit,
}

#[derive(Serialize)]
pub struct CreateMessageRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embeds: Option<Vec<Embed>>,
}

#[derive(Serialize)]
pub struct Embed {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

#[derive(Serialize)]
pub struct Field {
    pub name: String,
    pub value: String,
    pub inline: bool,
}

// X-RateLimit-Limit: 5
// X-RateLimit-Remaining: 0
// X-RateLimit-Reset: 1470173023
// X-RateLimit-Reset-After: 1
// X-RateLimit-Bucket: abcd1234
struct Ratelimit {
    remaining: u32,
}

impl Client {
    pub const fn new(config: Config) -> Self {
        Self {
            config,
            ratelimit: Ratelimit { remaining: 1 }
        }
    }

    fn add_headers(&self, request: Request) -> Request {
        let auth_payload = format!("Bot {}", self.config.token);
        request.set("Authorization", &auth_payload)
            .set("User-Agent", &self.config.user_agent)
    }

    fn update_ratelimits(&mut self, response: &Response) -> Result<(), Error> {
        self.ratelimit.remaining = response.header("X-RateLimit-Remaining")
            .ok_or(Error::MissingHeader("X-RateLimit-Remaining".to_owned()))?
            .parse().map_err(Error::ParseInt)?;

        log::info!("{} requests remaining", self.ratelimit.remaining);

        Ok(())
    }

    fn check_ratelimit(&self) -> Result<(), Error> {
        if self.ratelimit.remaining == 0 {
            return Err(Error::OutOfRequests);
        }

        Ok(())
    }

    // POST /channels/{channel.id}/messages
    pub fn create_message(&mut self, body: &CreateMessageRequest) -> Result<(), Error> {
        self.check_ratelimit()?;

        log::info!("create_message body {}", serde_json::to_string(body)?);
        let url = format!("{}channels/{}/messages", self.config.api_url, self.config.channel_id);
        let resp = self.add_headers(ureq::post(&url))
            .send_json(body)
            .map_err(Box::new)?;
        self.update_ratelimits(&resp)?;
        Ok(())
    }
}