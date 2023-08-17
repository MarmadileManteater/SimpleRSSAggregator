use std::{cmp::Ordering, fs::File};
use std::io::{Write, Read};
use serde::{Deserialize, Serialize};
use chrono::{NaiveDateTime, NaiveDate};
use regex::Regex;

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
struct MediaContent {
  #[serde(rename = "@url")]
  url: String,
  #[serde(rename(serialize = "media:description", deserialize = "media-description"))]
  description: String,
  #[serde(rename = "@type")]
  mime_type: String,
  #[serde(rename = "@fileSize")]
  file_size: String,
  #[serde(rename = "@medium")]
  medium: String
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
struct Item {
  guid: String,
  title: Option<String>,
  plain_title: Option<String>,
  imageurl: Option<String>,
  link: Option<String>,
  description: Option<String>,
  pub_date: Option<String>,
  create_date: Option<String>,
  update_date: Option<String>,
  #[serde(rename(serialize = "media:content", deserialize = "media-content"))]
  media_content: Option<Vec<MediaContent>>,
  #[serde(rename(serialize = "content:encoded", deserialize = "content-encoded"))]
  content_encoded: Option<String>
}
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
struct Channel {
  title: String,
  link: String,
  item: Vec<Item>,
}
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase", rename = "rss")]
struct Rss {
  channel: Channel,
  #[serde(rename = "@version")]
  version: Option<String>,
  #[serde(rename = "@xmlns:webfeeds")]
  webfeeds: Option<String>,
  #[serde(rename = "@xmlns:media")]
  media: Option<String>,
  #[serde(rename = "@xmlns:content")]
  content: Option<String>
}

fn clean_mastodon(input: &str) -> String {
  let re = Regex::new(r#"<(/?)([a-zA-Z_][a-zA-Z0-9_]*):([a-zA-Z_][a-zA-Z0-9_]*) *([^>]*)>"#).unwrap();
  format!("{}", re.replace_all(input, r#"<$1$2-$3 $4>"#))
}

fn clean_oga(input: &str) -> String {
  input.replace("<atom:link", "<atom-link")
}

fn clean_itch(input: &str) -> String {
  input.replace("& ", "&amp; ").replace("<description>", "<description><![CDATA[").replace("</description>", "]]></description>")
}

#[derive(Debug)]
enum FeedError {
  Reqwest(reqwest::Error),
  NonSuccessfulStatusCode(reqwest::StatusCode)
}

async fn fetch_feed(url: &str) -> Result<String, FeedError> {
  let client = reqwest::Client::new();
  let response = match client.get(url).send().await {
    Ok(result) => result,
    Err(err) => return Err(FeedError::Reqwest(err))
  };
  match response.status() {
    reqwest::StatusCode::OK => {
      response.text().await.map_err(|e| FeedError::Reqwest(e))
    },
    _ => {
      Err(FeedError::NonSuccessfulStatusCode(response.status()))
    }
  }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct ItemD {
    name: String,
    source: Option<String>,
}

#[tokio::main]
async fn main() {
  let feeds = ["https://marmadilemanteater.dev/blog/rss.xml", "https://gamemaking.social/@emma.rss", "https://programming.dev/feeds/u/emma.xml?sort=New", "https://opengameart.org/users/105608/art.xml", "https://itch.io/games/newest/by-marmadilemanteater.xml"];
  let mut rss = Vec::<Rss>::new();
  for feed_url in feeds {
    let feed = fetch_feed(feed_url).await;
    let feed_string = feed.unwrap();
    println!("{}", &clean_mastodon(&feed_string));
    let n = quick_xml::de::from_str::<Rss>(&clean_mastodon(&feed_string)).unwrap();
    rss.push(n);
  }
  let mut new_items = Vec::<Item>::new();
  for feed_channel in rss.clone() {
    let channel = feed_channel.channel;
    for item in channel.item {
      let r = Regex::new(r#"<[^>]*>"#).unwrap();
      
      let description = item.description.unwrap_or(String::from(""));
      let description_text = r.replace_all(&description, "").replace("&#39;", "'");
      let title = item.title.unwrap_or({
        let description_parts = description_text.split(" ").collect::<Vec::<&str>>();
        if description_parts.len() > 8 {
          format!("{}...", description_parts[0..8].join(" ").trim())
        } else {
          description_text
        }
      });
      new_items.push(Item {
        title: Some(title),
        description: Some(description),
        guid: item.guid,
        plain_title: item.plain_title,
        imageurl: item.imageurl,
        link: item.link,
        pub_date: item.pub_date,
        create_date: item.create_date,
        update_date: item.update_date,
        media_content: item.media_content,
        content_encoded: item.content_encoded
      });
    }
  }
  new_items.sort_by(|a, b| {
    let datetime = a.pub_date.clone().unwrap().replace("GMT", "+0000");
    let atimestamp = NaiveDateTime::parse_from_str(&datetime, "%a, %d %h %Y %H:%M:%S %z").unwrap().timestamp();
    let datetime = b.pub_date.clone().unwrap().replace("GMT", "+0000");
    let btimestamp = NaiveDateTime::parse_from_str(&datetime, "%a, %d %h %Y %H:%M:%S %z").unwrap().timestamp();
    if btimestamp > atimestamp {
      Ordering::Greater
    } else if atimestamp > btimestamp {
      Ordering::Less
    } else {
      Ordering::Equal
    }
  });
  let mut new_rss = Rss {
    channel: Channel {
      title: String::from("Emma <MarmadileManteater>"),
      link: String::from("https://maramadilemanteater.dev/feed"),
      item: new_items
    },
    version: Some(String::from("2.0")),
    webfeeds: Some(String::from("http://webfeeds.org/rss/1.0")),
    media: Some(String::from("http://search.yahoo.com/mrss/")),
    content: Some(String::from("http://purl.org/rss/1.0/modules/content/"))
  };

  //println!("{:#?}", &new_rss);
  let mut f = File::create("test.xml").unwrap();
  write!(f, r#"<?xml version="1.0" encoding="UTF-8"?>{}"#,quick_xml::se::to_string(&new_rss).unwrap_or(String::from("Failed")).replace("<content:encoded/>", ""));
}
