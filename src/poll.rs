use std::{thread, time::Duration};

use tokio::{sync::mpsc};

use crate::{config, error::Error, rule::{Rules, Rule}, models::{Post, Title}, reddit::{ListingResponse, self, ListingRequest}, db, discord::{self, CreateMessageRequest, Embed}};

pub async fn polling_loop(config: config::Config) -> Result<(), Error> {
    let mut db = db::Client::new(config.db);
    db.connect().await?;

    write_rules(&db, &config.rules).await?;

    // Reddit polling loop
    let (tx_post, mut rx_post) = mpsc::channel(32);
    tokio::spawn(async {
        poll_buildapcsales_forever(config.reddit, tx_post).await.unwrap();
    });

    // Process new posts and pass to notify loop
    let (tx_notify, mut rx_notify) = mpsc::channel(32);
    let tx_notify2 = tx_notify.clone();
    tokio::spawn(async move {
        process_posts(db, &mut rx_post, &tx_notify2, &config.rules).await.unwrap();
    });

    // Receive matches and notify user in batches
    notify_loop(config.discord, &mut rx_notify, &tx_notify).await?;

    Ok(())
}

async fn write_rules(db: &db::Client, rules: &Rules) -> Result<(), Error> {
    for rule in &rules.rules {
        db.insert_rule(rule).await?;
    }

    Ok(())
}

async fn write_posts(tx: &mpsc::Sender<Post>, listing: &ListingResponse) -> Result<(), Error> {
    for response_child in &listing.data.children {
        let post = &response_child.data;
        tx.send(post.clone())
            .await
            .map_err(|e| Error::Other(e.to_string()))?;
    }

    Ok(())
}

async fn poll_buildapcsales_forever(config: reddit::Config, tx: mpsc::Sender<Post>) -> Result<(), Error> {
    let mut reddit_client = reddit::Client::new(config);
    reddit_client.read_auth_from_file()?;
    
    loop {
        if reddit_client.is_auth_expired() {
            reddit_client.authenticate()?;
        }

        let listing = reddit_client.listing_new("buildapcsales", &ListingRequest {
            count: 0,
            limit: 10
        })?;

        write_posts(&tx, &listing).await?;

        let wait = reddit_client.get_wait_time();
        log::info!("Waiting for {}s", wait.as_secs());
        thread::sleep(wait);
    }
}

#[derive(Debug)]
pub struct MatchingPost {
    matching_rule: Rule,
    post: Post,
    title: Title,
}

async fn process_posts(db: db::Client, rx: &mut mpsc::Receiver<Post>, tx: &mpsc::Sender<NotifyMessage>, rules: &Rules) -> Result<(), Error> {
    loop {
        while let Some(post) = rx.recv().await {
            let is_new = db.insert_post(&post).await?;
            if !is_new {
                continue;
            }
            
            let Some(title) = Title::parse(&post.title, &post.id) else {
                continue;
            };

            let is_new = db.insert_parsed_title(&title).await?;
            if !is_new {
                continue;
            }

            let Some(matching_rule) = rules.get_matching_rule(&post, &title) else {
                continue;
            };

            let matching_post = MatchingPost {
                matching_rule,
                post,
                title,
            };
            db.insert_rule_match(&matching_post.post, &matching_post.matching_rule).await?;

            log::info!("Found match, sending to notify loop");
            tx.send(NotifyMessage::NewMatch(matching_post))
                .await
                .map_err(|e| Error::Other(e.to_string()))?;
        }
    }
}

#[derive(Debug)]
pub enum NotifyMessage {
    NewMatch(MatchingPost),
    TimerFired,
}

// async fn timer_task(id: u64, duration: Duration, tx: &mpsc::Sender<NotifyMessage>) -> Result<(), Error> {
//     log::info!("timer for {}s", duration.as_secs());
//     tokio::time::sleep(duration).await;
//     tx.send(NotifyMessage::TimerFired(id))
//         .await
//         .map_err(|e| Error::Other(e.to_string()))?;

//     Ok(())
// }

async fn clock(seconds: u64, tx: &mpsc::Sender<NotifyMessage>) -> Result<(), Error> {
    let mut interval = tokio::time::interval(Duration::from_secs(seconds));

    loop {
        tx.send(NotifyMessage::TimerFired).await
            .map_err(|e| Error::Other(format!("Send Error: {e}")))?;
        interval.tick().await;
    }
}

async fn notify_loop(config: discord::Config, rx: &mut mpsc::Receiver<NotifyMessage>, tx: &mpsc::Sender<NotifyMessage>) -> Result<(), Error> {
    let sending_interval_secs = config.sending_interval_secs;
    let mut discord_client = discord::Client::new(config);

    let tx2 = tx.clone();
    tokio::spawn(async move {
        clock(sending_interval_secs, &tx2).await.unwrap();
    });

    let mut queued_notifications: Vec<MatchingPost> = Vec::new();
    while let Some(msg) = rx.recv().await {
        log::info!("Received message on notify loop: {msg:?}");
        match msg {
            NotifyMessage::NewMatch(m) => {
                queued_notifications.push(m);
            },
            NotifyMessage::TimerFired => {
                if !queued_notifications.is_empty() {
                    notify(&queued_notifications, &mut discord_client)?;
                    queued_notifications.clear();
                }
            }
        }
    }

    Ok(())
}

fn notify(matches: &Vec<MatchingPost>, discord_client: &mut discord::Client) -> Result<(), Error> {
    log::warn!("Sending {} matches", matches.len());
    let body = matches_to_message_request(matches);
    log::debug!("{}", serde_json::to_string_pretty(&body)?);
    discord_client.create_message(&body)?;
    Ok(())
}

fn matches_to_message_request(matches: &Vec<MatchingPost>) -> CreateMessageRequest {
    let mut embeds = Vec::new();

    for post in matches {
        embeds.push(match_to_embed(post));
    }

    CreateMessageRequest {
        content: Some(format!("Found {} matches:", matches.len())),
        embeds: Some(embeds)
    }
}

fn match_to_embed(m: &MatchingPost) -> Embed {
    Embed { 
        title: Some(m.matching_rule.name()),
        description: Some(m.post.title.clone()),
        url: Some(m.post.get_comments_url()), 
    }
}
