use std::{str::FromStr, hash::Hash, collections::hash_map::DefaultHasher};

use serde::Deserialize;
use sqlx::{Sqlite, SqlitePool, migrate::MigrateDatabase, sqlite::SqliteConnectOptions, ConnectOptions};

use crate::{error::Error, models::{Post, Title}, rule};

#[derive(Deserialize, Debug, PartialEq)]
pub struct Config {
    pub db_url: String,
}

pub struct Client {
    config: Config,
    db: Option<SqlitePool>,
}

impl Client {
    pub fn new(config: Config) -> Self {
        Client {
            config,
            db: None,
        }
    }

    pub async fn connect(&mut self) -> Result<(), Error> {
        log::info!("Connecting to database at {}", self.config.db_url);
        let options = SqliteConnectOptions::from_str(&self.config.db_url)?
            .disable_statement_logging().clone();
        let db = SqlitePool::connect_with(options).await?;
        self.db = Some(db);
        Ok(())
    }

    fn get_db(&self) -> Result<&SqlitePool, Error> {
        let Some(db) = &self.db else {
            return Err(Error::Other("db is null".to_owned()));
        };
        Ok(db)
    }

    pub async fn setup(&mut self) -> Result<(), Error> {
        log::info!("Creating database at {}", self.config.db_url);
        if Sqlite::database_exists(&self.config.db_url).await.unwrap_or(false) {
            log::info!("Database already exists");
        } else {
            match Sqlite::create_database(&self.config.db_url).await {
                Ok(_) => log::info!("Successfully created database"),
                Err(error) => panic!("error: {error}"),
            }
        }

        self.connect().await?;
        let db = self.get_db()?;
    
        log::info!("Running migrations");
        let crate_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let migrations = std::path::Path::new(&crate_dir).join("./migrations");
    
        let migration_results = sqlx::migrate::Migrator::new(migrations)
            .await
            .unwrap()
            .run(db)
            .await;
    
        match migration_results {
            Ok(_) => log::info!("Successfully ran migrations"),
            Err(error) => {
                panic!("error: {error}");
            }
        }
        Ok(())
    }

    pub async fn insert_post(&self, post: &Post) -> Result<bool, Error> {
        let db = self.get_db()?;
        let response = sqlx::query(
            "INSERT OR IGNORE INTO posts (id, created_utc, downs, link_flair_text, title, ups, url)
                  VALUES (?, ?, ?, ?, ?, ?, ?)")
            .bind(&post.id)
            .bind(post.created_utc)
            .bind(post.downs)
            .bind(&post.link_flair_text)
            .bind(&post.title)
            .bind(post.ups)
            .bind(&post.url)
            .execute(db)
            .await?;   
    
        Ok(response.rows_affected() > 0)
    }

    pub async fn insert_parsed_title(&self, title: &Title) -> Result<bool, Error> {
        let db = self.get_db()?;
        let response = sqlx::query(
            "INSERT OR IGNORE INTO parsed_titles (post_id, product_type, description, price_dollars, price_cents, extra_details)
                VALUES (?, ?, ?, ?, ?, ?)")
                .bind(&title.post_id)
                .bind(&title.product_type)
                .bind(&title.description)
                .bind(title.price_dollars)
                .bind(title.price_cents)
                .bind(&title.extra_details)
                .execute(db)
                .await?;

        Ok(response.rows_affected() > 0)
    }    

    pub async fn insert_rule(&self, rule: &rule::Rule) -> Result<bool, Error> {
        let db = self.get_db()?;
        let response = sqlx::query(
            "INSERT OR IGNORE INTO rules (id, name, link_flair_pattern, product_type_pattern, description_pattern, price_min, price_max)
                 VALUES (?, ?, ?, ?, ?, ?, ?)")
                 .bind(rule.hash())
                 .bind(&rule.name)
                 .bind(rule.link_flair_pattern.as_ref().map(|p| &p.source))
                 .bind(rule.product_type_pattern.as_ref().map(|p| &p.source))
                 .bind(rule.description_pattern.as_ref().map(|p| 
                    &p.source))
                 .bind(rule.price_min_dollars)
                 .bind(rule.price_max_dollars)
                 .execute(db)
                 .await?;

        Ok(response.rows_affected() > 0)
    }

    pub async fn insert_rule_match(&self, post: &Post, rule: &rule::Rule) -> Result<bool, Error> {
        let db = self.get_db()?;
        let response = sqlx::query(
            "INSERT OR IGNORE INTO rule_matches (rule_id, post_id, created_utc)
            VALUES (?, ?, ?)")
            .bind(rule.hash())
            .bind(&post.id)
            .bind(chrono::Utc::now().to_rfc3339())
            .execute(db)
            .await?;

        Ok(response.rows_affected() > 0)
    }
}

