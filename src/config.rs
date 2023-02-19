use std::fs;

use serde::Deserialize;

use crate::{rule, reddit, discord, sms, error::Error, db};

#[derive(Deserialize, PartialEq)]
pub struct Config {
    #[serde(skip_deserializing)]
    pub rules: rule::Rules,
    #[serde(rename = "rules")]
    rules_internal: Vec<rule::Rule>,
    pub reddit: reddit::Config,
    pub discord: discord::Config,
    pub twilio: sms::Config,
    pub db: db::Config,
}

impl Config {
    pub fn read_from_toml_file(filename: &str) -> Result<Config, Error> {
        let contents = fs::read_to_string(filename)?;
        
        Self::from_toml(&contents)
    }

    pub fn from_toml(source: &str) -> Result<Config, Error> {
        let mut config: Self = toml::from_str(&source).map_err(|e| Error::Toml(e))?;
        config.rules = rule::Rules {
            rules: config.rules_internal.clone()
        };

        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use crate::rule::{PatternAndSource, Pattern};

    use super::*;

    #[test]
    fn test_parse_config_toml() {
        let toml_source = 
r#"
[[rules]]
name = "Rule name"
product_type_pattern = "\"Rule product\""
description_pattern = "term1 || term2"

[reddit]
auth_host = "https://www.reddit.com/api/v1/"
api_host = "https://oauth.reddit.com/"
token_file = "token.json"
username = "<YOUR USERNAME>"
password = "<YOUR USER'S PASSWORD>"
client_id = "<YOUR API CLIENT ID>"
client_secret = "<YOUR API CLIENT SECRET>"
user_agent = "<YOUR API CLIENT USER AGENT>"
wait_time_secs = 5

[discord]
token = "<YOUR DISCORD BOT TOKEN>"
user_agent = "<YOUR DISCORD BOT USER AGENT>"
api_url = "https://discord.com/api/v10/"
channel_id = "<YOUR DISCORD CHANNEL ID TO POST MESSAGES TO>"
sending_interval_secs = 10

[twilio]
api_url = "https://api.twilio.com/2010-04-01/Accounts/"
api_key = "<YOUR API KEY>"
api_key_secret = "<YOUR ACCOUNT KEY SECRET>"
account_sid = "<YOUR ACCOUNT SID>"
phone_number_from = "<NUMBER TO SEND FROM>"
phone_number_to = "<NUMBER TO SEND TO>"

[db]
db_url = "sqlite://sqlite.db"
"#;
        
        let parsed = Config::from_toml(toml_source);
        assert!(parsed.is_ok());
        let parsed: Config = parsed.unwrap();

        assert_eq!(parsed.reddit, reddit::Config { 
            auth_host: "https://www.reddit.com/api/v1/".to_owned(), 
            api_host: "https://oauth.reddit.com/".to_owned(), 
            token_file: "token.json".to_owned(), 
            username: "<YOUR USERNAME>".to_owned(), 
            password: "<YOUR USER'S PASSWORD>".to_owned(), 
            client_id: "<YOUR API CLIENT ID>".to_owned(),
            client_secret: "<YOUR API CLIENT SECRET>".to_owned(),
            user_agent: "<YOUR API CLIENT USER AGENT>".to_owned(), 
            wait_time_secs: 5 
        });

        assert_eq!(parsed.discord, discord::Config { 
            token: "<YOUR DISCORD BOT TOKEN>".to_owned(),
            user_agent: "<YOUR DISCORD BOT USER AGENT>".to_owned(),
            api_url: "https://discord.com/api/v10/".to_owned(),
            channel_id: "<YOUR DISCORD CHANNEL ID TO POST MESSAGES TO>".to_owned(),
            sending_interval_secs: 10
        });

        assert_eq!(parsed.rules, rule::Rules {
            rules: vec![
                rule::Rule {
                    name: Some("Rule name".to_owned()),
                    description_pattern: Some(PatternAndSource {
                        source: "term1 || term2".to_owned(),
                        pattern: Pattern::Or(
                            Box::new(Pattern::Exact("term1".to_owned())),
                            Box::new(Pattern::Exact("term2".to_owned()))
                        )
                    }),
                    product_type_pattern: Some(PatternAndSource {
                        source: "\"Rule product\"".to_owned(),
                        pattern: Pattern::Exact("Rule product".to_owned())
                    }),
                    link_flair_pattern: None,
                    price_max_dollars: None,
                    price_min_dollars: None
                }
            ]
        })
    }
}