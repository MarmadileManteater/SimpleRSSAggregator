use std::{collections::HashMap, cmp::Ordering};

use chrono::{NaiveDateTime, TimeZone};
use serde::{Deserialize, Serialize};
use serde_json::Map;
use substring::Substring;

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MediaContent {
  #[serde(rename = "@url")]
  pub url: String,
  #[serde(alias = "media:description", rename(serialize = "media:description", deserialize = "media-description"))]
  pub description: Option<String>,
  #[serde(rename = "@type")]
  mime_type: String,
  #[serde(rename = "@fileSize")]
  file_size: Option<String>,
  #[serde(rename = "@medium")]
  medium: String
}

impl MediaContent {
  pub fn into_html(&self) -> String {
    let url = &self.url;
    let description = &self.description.clone().unwrap_or(String::from(""));
    if self.mime_type.starts_with("image") {
      format!("<img src=\"{url}\" alt=\"{description}\" />")
    } else if self.mime_type.starts_with("video") {
      let r#type = &self.mime_type;
      format!("<video src=\"{url}\" type=\"{type}\" controls>{description}</video>")
    } else {
      format!("{description}")
    }
  }
}

pub trait ContainsMedia {
  fn into_html(&self) -> Vec::<String>;
}

impl ContainsMedia for Option<Vec<MediaContent>> {
  fn into_html(&self) -> Vec::<String> {
    match &self {
      Some(media_content) => {
        media_content.into_iter().map(|c| c.into_html()).collect::<Vec::<String>>()
      },
      None => {
        vec![]
      }
    }
  }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Item {
  pub guid: String,
  pub title: Option<String>,
  pub plain_title: Option<String>,
  pub imageurl: Option<String>,
  pub link: Option<String>,
  pub description: Option<String>,
  pub author: Option<Author>,
  pub pub_date: Option<String>,
  pub create_date: Option<String>,
  pub update_date: Option<String>,
  #[serde(alias = "media:content", rename(serialize = "media:content", deserialize = "media-content"))]
  pub media_content: Option<Vec<MediaContent>>,
  #[serde(alias = "content:encoded", rename(serialize = "content:encoded", deserialize = "content-encoded"))]
  pub content_encoded: Option<String>
}

fn get_timestamp_from_string(string: Option<String>, fmt_string: &str) -> Option<i64> {
  string
    .map(|d| {
        Some(NaiveDateTime::parse_from_str(&d, fmt_string)
              .map(|dt| Some(dt))
              .unwrap_or(None)
              .map(|dt| dt.timestamp()))
    })
    .unwrap_or(None)
    .unwrap_or(None)
}

impl Item {
  pub fn get_created_timestamp(&self) -> Option<i64> {
    get_timestamp_from_string(self.create_date.as_ref().map(|d| d.replace("GMT", "+0000")), "%a, %d %h %Y %H:%M:%S %z")
  }
  pub fn get_updated_timestamp(&self) -> Option<i64> {
    get_timestamp_from_string(self.update_date.as_ref().map(|d| d.replace("GMT", "+0000")), "%a, %d %h %Y %H:%M:%S %z")
  }
  pub fn get_published_timestamp(&self) -> Option<i64> {
    get_timestamp_from_string(self.pub_date.as_ref().map(|d| d.replace("GMT", "+0000")), "%a, %d %h %Y %H:%M:%S %z")
  }
  pub fn update(&mut self, new_item: Item) {
    self.title = new_item.title;
    self.plain_title = new_item.plain_title;
    self.description = new_item.description;
    self.imageurl = new_item.imageurl;
    self.content_encoded = new_item.content_encoded;
    self.media_content = new_item.media_content;
    self.update_date = new_item.update_date;
    self.pub_date = new_item.pub_date;
    self.create_date = new_item.create_date;
  }
}

pub trait CombineItemLists {
  fn update_list_by_guids(&mut self, new_items: Vec::<Item>);
}

impl CombineItemLists for Vec::<Item> {
  fn update_list_by_guids(&mut self, new_items: Vec::<Item>) {
    for new_item in new_items {
      let mut is_new = true;
      for item in &mut *self {
        if new_item.guid == item.guid {
          is_new = false;
          item.update(new_item.clone());
        }
      }
      if is_new {
        self.push(new_item);
      }
    }
  }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Channel {
  pub title: String,
  pub link: String,
  pub item: Vec<Item>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase", rename = "rss")]
pub struct Rss {
  pub channel: Channel,
  #[serde(rename = "@version")]
  pub version: Option<String>,
  #[serde(rename = "@xmlns:webfeeds")]
  pub webfeeds: Option<String>,
  #[serde(rename = "@xmlns:media")]
  pub media: Option<String>,
  #[serde(rename = "@xmlns:content")]
  pub content: Option<String>
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct FeedOptions {
  pub rss: Rss,
  // cmd to pass input into and accept output out of
  pub manipulate_input: String,
  pub retain_all_entries: bool,
  pub title: String,
  pub link: String
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Db {
  pub rss: HashMap::<String, FeedOptions>,
  pub output_one_channel: bool,
  pub title: String,
  pub link: String,
  // useful for mastodon because mastodon doesn't populate the title
  // and RSS readers don't always display posts without titles kindly
  pub include_description_as_title_if_none_given: bool,
  // the number of words to be put into the title from the description
  // if there is no title
  pub description_title_word_count: i32,
  pub title_ellipsis: String,
  pub populate_content_encoded: bool,
  pub add_media_to_content_encoded: bool,
  pub max_entries_published: i32,
  pub override_item_author: bool
}

impl Db {
  pub fn new() -> Db {
    Db {
      rss: HashMap::<String, FeedOptions>::new(),
      output_one_channel: true,
      title: String::from(""),
      link: String::from(""),
      include_description_as_title_if_none_given: true,
      description_title_word_count: 10,
      title_ellipsis: String::from("..."),
      populate_content_encoded: true,
      add_media_to_content_encoded: true,
      max_entries_published: -1,// -1 is max
      override_item_author: false
    }
  }
  pub fn output_rss(&self) -> Result<std::string::String, quick_xml::DeError> {
    let mut items = Vec::<Item>::new();
    for (_, feed_options) in self.rss.clone() {
      for mut item in feed_options.rss.channel.item {
        if item.author.is_none() || self.override_item_author {
          item.author = Some(Author {
            name: feed_options.title.clone(),
            uri: feed_options.link.clone()
          });
        }
        if item.title.is_none() && self.include_description_as_title_if_none_given {
          item.title = item.description.clone().map(|d| {
            let r = regex::Regex::new(r#"<[^>]*>"#).unwrap();
            let d_text = r.replace_all(&d, "").replace("&#39;", "'");
            let parts = d_text.split(" ").collect::<Vec::<&str>>();
            if parts.len() > self.description_title_word_count as usize {
              format!("{}{}", parts[0..self.description_title_word_count as usize].join(" ").trim(), self.title_ellipsis)
            } else {
              d_text
            }
          });
        }
        if item.content_encoded.is_none() && self.populate_content_encoded {
          item.content_encoded = item.description.clone();
        }
        if item.content_encoded.is_some() && self.add_media_to_content_encoded {
          let content_encoded = item.content_encoded.clone().unwrap();
          item.content_encoded = Some(format!("{}<br>{}", content_encoded, item.media_content.clone().map(|mc| {
            mc.into_iter().filter_map(|c| {
              if !content_encoded.contains(&c.url) {
                Some(c.into_html())
              } else {
                None
              }
            }).collect::<Vec<_>>().join(" ")
          }).unwrap_or("".to_owned())));
        }
        items.push(item);
      }
    }
    items.sort_by(|a, b| {
      let atime = a.get_published_timestamp();
      let btime = b.get_published_timestamp();
      if atime > btime { 
        Ordering::Less
      } else if atime < btime {
        Ordering::Greater
      } else {
        Ordering::Equal
      }
    });
    if self.max_entries_published > 0 {
      items = items[0..self.max_entries_published as usize].to_vec()
    }
    let new_rss = Rss {
      channel: Channel {
        title: self.title.clone(),
        link: self.link.clone(),
        item: items
      },
      version: Some(String::from("2.0")),
      webfeeds: Some(String::from("http://webfeeds.org/rss/1.0")),
      media: Some(String::from("http://search.yahoo.com/mrss/")),
      content: Some(String::from("http://purl.org/rss/1.0/modules/content/"))
    };
    quick_xml::se::to_string(&new_rss)
  }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Link {
  #[serde(rename = "@rel")]
  pub rel: String,
  #[serde(rename = "@href")]
  pub href: String
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Entry {
  pub id: String,
  pub title: String,
  pub author: Author,
  pub updated: String,
  pub content: String,
  #[serde(alias = "media:content", rename(serialize = "media:content", deserialize = "media-content"))]
  pub media_content: Option<Vec<MediaContent>>,
  pub link: Link,
  pub summary: String
}

impl Entry {
  pub fn into_item(&self) -> Item {
    let pub_date = self.get_updated_time_as_item_format();
    Item {
      guid: self.id.clone(),
      title: Some(self.title.clone()),
      plain_title: Some(self.title.clone()),
      imageurl: None,
      link: Some(self.link.href.clone()),
      description: Some(self.summary.clone()),
      pub_date: pub_date.clone(),
      create_date: None,
      update_date: pub_date.clone(),
      media_content: self.media_content.clone(),
      content_encoded: Some(self.content.clone()),
      author: Some(self.author.clone())
    }
  }
  pub fn get_updated_time_as_item_format(&self) -> Option<String> {
    let ts = get_timestamp_from_string(Some(self.updated.clone()), "%Y-%m-%dT%T%.3fZ");
    ts.map(|timestamp| {
      let datetime = chrono::Utc.timestamp_opt(timestamp, 0).unwrap();
      datetime.format("%a, %d %h %Y %H:%M:%S %z").to_string()
    })
  }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Author {
  pub name: String,
  pub uri: String
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Feed {
  pub id: String,
  pub title: String,
  pub subtitle: String,
  pub entry: Option<Vec::<Entry>>,
  pub author: Author,
  pub updated: Option<String>
}

impl Feed {
  pub fn into_rss(&self) -> Rss {
    Rss {
      channel: Channel {
        title: self.title.clone(),
        link: self.author.uri.clone(),
        item: self.entry.clone().unwrap_or(vec![]).into_iter().map(|e| e.into_item()).collect()
      },
      version: Some(String::from("2.0")),
      webfeeds: Some(String::from("http://webfeeds.org/rss/1.0")),
      media: Some(String::from("http://search.yahoo.com/mrss/")),
      content: Some(String::from("http://purl.org/rss/1.0/modules/content/"))
    }
  }
}
