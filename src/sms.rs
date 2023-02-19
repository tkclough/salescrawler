use std::fs;

use serde::Deserialize;
use url::Url;

use crate::{auth::make_basic_auth_header, error::Error};

#[derive(Deserialize, PartialEq)]
pub struct Config {
    api_url: String,
    api_key: String,
    api_key_secret: String,
    account_sid: String,
    phone_number_from: String,
    phone_number_to: String,
}

pub struct Client {
    config: Config,
}

#[derive(Deserialize, Debug)]
pub struct SendMessageResponseBody {
    pub uri: String,
}
impl Client {
    pub const fn new(config: Config) -> Self {
        Self {
            config
        }
    }

    // EXCLAMATION_MARK='!'
    // curl -X POST "https://api.twilio.com/2010-04-01/Accounts/$TWILIO_ACCOUNT_SID/Messages.json" \
    // --data-urlencode "Body=Hello there$EXCLAMATION_MARK" \
    // --data-urlencode "From=+15555555555" \
    // --data-urlencode "MediaUrl=https://demo.twilio.com/owl.png" \
    // --data-urlencode "To=+12316851234" \
    // -u $TWILIO_ACCOUNT_SID:$TWILIO_AUTH_TOKEN

    pub fn send_message(&self, body: &str) -> Result<SendMessageResponseBody, Error> {
        let uri = format!("{}/Messages.json", self.config.account_sid);
        let api_url = Url::parse(self.config.api_url.as_str())?.join(&uri)?;

        let auth = make_basic_auth_header(&self.config.api_key, &self.config.api_key_secret);

        let body = format!("Body={}&From={}&To={}",
            &urlencoding::encode(body).to_string(),
            &urlencoding::encode(&self.config.phone_number_from).to_string(),
            &urlencoding::encode(&self.config.phone_number_to).to_string());

        let resp = ureq::post(api_url.as_str())
            .set("Authorization", &auth)
            .set("Content-Type", "application/x-www-form-urlencoded")
            .send_string(&body).map_err( Box::new)?;

        let body = resp.into_string()?;
        let parsed: SendMessageResponseBody = serde_json::from_str(&body)?;
        
        Ok(parsed)
    }
}

// {
//     "account_sid": "ACXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX",
//     "api_version": "2010-04-01",
//     "body": "Hello there!",
//     "date_created": "Thu, 30 Jul 2015 20:12:31 +0000",
//     "date_sent": "Thu, 30 Jul 2015 20:12:33 +0000",
//     "date_updated": "Thu, 30 Jul 2015 20:12:33 +0000",
//     "direction": "outbound-api",
//     "error_code": null,
//     "error_message": null,
//     "from": "+15555555555",
//     "messaging_service_sid": "MGXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX",
//     "num_media": "0",
//     "num_segments": "1",
//     "price": null,
//     "price_unit": null,
//     "sid": "SMXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX",
//     "status": "sent",
//     "subresource_uris": {
//         "media": "/2010-04-01/Accounts/ACXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX/Messages/SMXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX/Media.json"
//     },
//     "to": "+12316851234",
//     "uri": "/2010-04-01/Accounts/ACXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX/Messages/SMXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX.json"
// }