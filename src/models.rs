use regex::Regex;
use serde::Deserialize;
use sqlx::FromRow;

use crate::rule::{self};

#[derive(Clone, Deserialize, Debug)]
pub struct Post {
    pub created_utc: f64,
    pub downs: f64,
    pub link_flair_text: Option<String>,
    pub title: String,
    pub ups: f64,
    pub url: String,
    pub id: String,
}

impl Post {
    pub fn get_comments_url(&self) -> String {
        format!("https://www.reddit.com/r/buildapcsales/comments/{}", self.id)
    }
}

impl rule::Subject for Post {
    fn is_match(&self, rule: &rule::Rule) -> bool {
        match &rule.link_flair_pattern {
            Some(link_flair_pattern) => 
                link_flair_pattern.pattern.does_string_option_match(&self.link_flair_text),
            _ => true
        }
    }
}

#[derive(Deserialize, Debug, PartialEq, Eq)]
pub struct Title {
    pub post_id: String,
    pub product_type: String,
    pub description: String,
    pub price_dollars: i32,
    pub price_cents: i8,
    pub extra_details: Option<String>,
}

impl Title {
    pub fn parse(title: &str, post_id: &str) -> Option<Self> {
        let re = Regex::new(r"\[(?P<type>[ \w]+)\](?P<desc>[^$]*)\$(?P<price_dollars>\d+)(\.(?P<price_cents>\d+))?(?P<extra>[^\d].*)?").ok()?;
        match re.captures(title) {
            Some(m) => {
                let product_type = m.name("type")?.as_str().trim().to_owned();
                let description = m.name("desc")?.as_str().trim().to_owned();

                let price_dollars = m.name("price_dollars")?;
                let price_dollars = price_dollars.as_str().parse().ok()?;

                let price_cents = m.name("price_cents");
                let price_cents: i8 = price_cents.map_or(Some(0), |price_cents| {
                    price_cents.as_str().parse().ok()
                })?;

                let extra_details = m.name("extra").map(|s| s.as_str().trim().to_owned());

                Some(Self {
                    post_id: post_id.to_owned(),
                    product_type,
                    description,
                    price_dollars,
                    price_cents,
                    extra_details,
                })
            }
            _ => None,
        }
    }

    fn price(&self) -> f64 {
        (self.price_dollars as f64) + 0.1 * (self.price_cents as f64)
    }
}

impl rule::Subject for Title {
    fn is_match(&self, rule: &rule::Rule) -> bool {
        if let Some(ref product_type_pattern) = rule.product_type_pattern {
            if !product_type_pattern.pattern.does_string_match(&self.product_type) {
                return false;
            }
        }

        if let Some(ref description_pattern) = rule.description_pattern {
            if !description_pattern.pattern.does_string_match(&self.description) {
                return false;
            }
        }

        if let Some(ref price_min) = rule.price_min_dollars {
            if *price_min as f64 > self.price() {
                return false;
            }
        }

        if let Some(ref price_max) = rule.price_max_dollars {
            if self.price() > *price_max as f64 {
                return false;
            }
        }

        true    
    }
}

#[derive(FromRow)]
pub struct Rule {
    pub id: u64,
    pub name: Option<String>,
    pub link_flair_pattern: Option<String>,
    pub product_type_pattern: Option<String>,
    pub description_pattern: Option<String>,
    pub price_min: Option<f64>,
    pub price_max: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_title_1() {
        let title = "[GPU] ASUS - NVIDIA GeForce RTX 4070 Ti TUF 12GB GDDR6X PCI Express 4.0 Graphics Card - Black $799.99";
        let expected = Title {
            post_id: "1234".to_owned(),
            product_type: "GPU".to_owned(),
            description: "ASUS - NVIDIA GeForce RTX 4070 Ti TUF 12GB GDDR6X PCI Express 4.0 Graphics Card - Black".to_owned(),
            price_dollars: 799,
            price_cents: 99,
            extra_details: None
        };

        let parsed = Title::parse(title, "1234");
        assert!(parsed.is_some());
        let parsed = parsed.unwrap();

        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_parse_title_2() {
        let title = "[MOBO] ASUS TUF GAMING B650M-PLUS WIFI AM5 Ryzen 7000 mATX gaming motherboard(14 power stages, PCIe 5.0 M.2 support, DDR5 memory, 2.5 Gb Ethernet, WiFi 6, USB4 support and Aura Sync) $196 FS";
        let expected = Title {
            post_id: "1234".to_owned(),
            product_type: "MOBO".to_owned(),
            description: "ASUS TUF GAMING B650M-PLUS WIFI AM5 Ryzen 7000 mATX gaming motherboard(14 power stages, PCIe 5.0 M.2 support, DDR5 memory, 2.5 Gb Ethernet, WiFi 6, USB4 support and Aura Sync)".to_owned(),
            price_dollars: 196,
            price_cents: 0,
            extra_details: Some("FS".to_owned())
        };

        let parsed = Title::parse(title, "1234");
        assert!(parsed.is_some());
        let parsed = parsed.unwrap();

        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_parse_title_3() {
        let title = "[PSU] Corsair HX1000 80+ Platinum - $163.19 ($254.99-$91.80) MICROCENTER IN STORE ONLY";
        let expected = Title {
            post_id: "1234".to_owned(),
            product_type: "PSU".to_owned(),
            description: "Corsair HX1000 80+ Platinum -".to_owned(),
            price_dollars: 163,
            price_cents: 19,
            extra_details: Some("($254.99-$91.80) MICROCENTER IN STORE ONLY".to_owned()),
        };

        let parsed = Title::parse(title, "1234");
        assert!(parsed.is_some());
        let parsed = parsed.unwrap();

        assert_eq!(parsed, expected);
    }
}