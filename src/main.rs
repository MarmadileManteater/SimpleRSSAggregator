
mod structs;

use std::path::Path;
use std::{cmp::Ordering, fs::File};
use std::io::{Write, Read};
use regex::Regex;
use structs::*;

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

impl std::fmt::Display for FeedError {
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
     match self {
        FeedError::NonSuccessfulStatusCode(code) => write!(f, "Request returned non-successful status code: {}", code),
        FeedError::Reqwest(error) => write!(f, "Error making request: {}", error)
     }
  }
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

#[derive(Debug)]
enum DbCreateError {
  FileCreateError(std::io::Error),
  FormattingError(serde_json::error::Error),
  FileWriteError(std::io::Error)
}

impl std::fmt::Display for DbCreateError {
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
     match self {
        DbCreateError::FileCreateError(error) => write!(f, "Error creating file: {}", error),
        DbCreateError::FormattingError(error) => write!(f, "Error formatting file: {}", error),
        DbCreateError::FileWriteError(error) => write!(f, "Error writing to file: {}", error)
     }
  }
}

fn save_db(db: &Db, path: &str) -> Result<(),DbCreateError> {
  let mut file = match File::create(path) {
    Ok(result) => result,
    Err(error) => {
      return Err(DbCreateError::FileCreateError(error));
    }
  };
  let formatted_db = match serde_json::to_string_pretty(&db) {
    Ok(db) => db,
    Err(error) => {
      return Err(DbCreateError::FormattingError(error))
    }
  };
  write!(file, "{}", formatted_db)
    .map_err(|error| DbCreateError::FileWriteError(error))
}

fn create_db(path: &str) -> Result<(),DbCreateError> {
  let db = Db::new();
  save_db(&db, path)
}

fn create_db_if_none_exists(path: &str, log: bool) -> Result<bool, DbCreateError> {
  if !Path::new(path).exists() {
    match create_db(path) {
      Ok(()) => {
        if log {
          log::info!("✅ Sucessfully created db!");
        }
        Ok(true)
      },
      Err(error) => {
        log::error!("❌ {}", error);
        Err(error)
      }
    }
  } else {
    if log {
      log::info!("➡ Skipping because db exists!")
    }
    Ok(false)
  }
}

#[derive(Debug)]
enum GetDbError {
  FileOpenError(std::io::Error),
  FileReadError(std::io::Error),
  JsonDeserializeError(serde_json::error::Error)
}

impl std::fmt::Display for GetDbError {
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
     match self {
        GetDbError::FileOpenError(error) => write!(f, "Error opening file: {}", error),
        GetDbError::FileReadError(error) => write!(f, "Error reading file: {}", error),
        GetDbError::JsonDeserializeError(error) => write!(f, "Error deserializing JSON from file: {}", error)
     }
  }
}

fn get_db(path: &str) -> Result<Db, GetDbError> {
  let mut f = match File::open(path) {
    Ok(f) => f,
    Err(error) => {
      return Err(GetDbError::FileOpenError(error))
    }
  };
  let mut output = String::from("");
  match f.read_to_string(&mut output) {
    Ok(_) => {
      match serde_json::from_str::<Db>(&output) {
        Ok(result) => {
          Ok(result)
        },
        Err(error) => {
          Err(GetDbError::JsonDeserializeError(error))
        }
      }
    },
    Err(error) => {
      Err(GetDbError::FileReadError(error))
    }
  }
}

use futures_util::StreamExt;

async fn download_image(url: &str) -> Result<String, ()> {
  let client = reqwest::Client::new();
  match client.get(url).send().await {
    Ok(response) => {
      let file_name = url.split("/").last().unwrap_or("untitled");
      let mut file = File::create(format!("output/media/{}", file_name)).unwrap();
      let mut stream = response.bytes_stream();
      while let Some(chunk) = stream.next().await {
        match chunk {
          Ok(chunk) => {
            file.write(&chunk);
          },
          Err(err) => {
            break;
          }
        }
      }
      log::info!("Finished downloading file: {file_name}");
      Ok(file_name.to_string())
    },
    Err(error) => {
      Err(())
    }
  }
}

#[tokio::main]
async fn main() {
  env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));
  let mut db = match get_db("db.json") {
    Ok(db) => db,
    Err(error) => {
      log::error!("❌ {}", error);
      Db::new()
    }
  };
  let args: Vec<String> = std::env::args().collect();
  let pkg_version = env!("CARGO_PKG_VERSION");
  println!("Syndication Junction v{pkg_version}");
  if args.len() > 1 {
    match args[1].as_str() {
      "fetch" => {
        if args.len() > 2 {
          let feeds = args[2..args.len()].to_vec();
          for feed in feeds {
            match fetch_feed(&feed).await {
              Ok(feed_str) => {
                let rss = match quick_xml::de::from_str::<Rss>(&clean_mastodon(&feed_str)) {
                  Ok(rss) => {
                    Some(rss)
                  },
                  Err(error) => {
                    match quick_xml::de::from_str::<Feed>(&clean_mastodon(&feed_str)) {
                      Ok(feed) => {
                        Some(feed.into_rss())
                      },
                      Err(derror) => {
                        log::error!("❌ No matching format found for the RSS file: {} {}", error, derror);
                        None
                      }
                    }
                  }
                };
                match rss {
                  Some(rss) => {
                    let feed_options = if db.rss.contains_key(&feed) {
                      let a = db.rss.get(&feed).map(|e|Some(e.to_owned()));
                      db.rss.remove(&feed);
                      a.map(|a| a.map(|mut options| {
                        options.rss.channel.item.update_list_by_guids(rss.channel.item.clone());
                        options.rss.channel.title = rss.channel.title.clone();
                        options.rss.channel.link = rss.channel.link.clone();
                        Some(options)
                      })).unwrap_or(None).unwrap_or(None)
                    } else {
                      Some(FeedOptions {
                        rss: rss.clone(),
                        manipulate_input: "".to_string(),
                        retain_all_entries: true,
                        title: rss.channel.title.clone(),
                        link: rss.channel.link.clone()
                      })
                    };
                    match feed_options {
                      Some(options) => {
                        db.rss.insert(feed, options);
                      },
                      None => {}
                    }
                  },
                  None => {}
                }
              },
              Err(err) => {
                log::error!("❌ {}", err);
              }
            }
          }
          match save_db(&db, "db.json") {
            Ok(()) => {
              log::info!("Db sucessfully saved!");
            },
            Err(error) => {
              log::error!("❌ {}", error);
            }
          }
        }
      },
      "output-rss" => {
        let mut f = File::create("test.xml").unwrap();
        write!(f, r#"<?xml version="1.0" encoding="UTF-8"?>{}"#, db.output_rss().unwrap_or(String::from("Failed")).replace("<content:encoded/>", ""));
      },
      _ => {
        
      }
    }
  }

  println!("{:#?}", args);
  
  /* 
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
      let title = item.title.clone().unwrap_or({
        let description_parts = description_text.split(" ").collect::<Vec::<&str>>();
        if description_parts.len() > 8 {
          format!("{}...", description_parts[0..8].join(" ").trim())
        } else {
          description_text
        }
      });
      let mut guid_already_exists = false;
      let d = &description;
      let t = &title;
      for i in 0..new_items.len() {
        if new_items[i].link == item.link {
          println!("{:#?} == {:#?}", new_items[i].title, &item.title);
          guid_already_exists = true;
          if new_items[i].description.clone().unwrap_or(String::from("")).len() < d.len() {
            new_items[i].description = Some(String::from(d));
            new_items[i].title = Some(String::from(t));
          }
        }
      }
      if guid_already_exists {
        continue;
      }
      new_items.push(Item {
        title: Some(title),
        description: Some(description.clone()),
        guid: item.guid,
        plain_title: item.plain_title,
        imageurl: item.imageurl,
        link: item.link,
        pub_date: item.pub_date,
        create_date: item.create_date,
        update_date: item.update_date,
        media_content: item.media_content.clone(),
        content_encoded: Some(item.content_encoded.unwrap_or(format!("{}<br/>{}", description, item.media_content.into_html().join(" "))))
      });
    }
  }
  new_items.sort_by(|a, b| {
    let atimestamp = a.get_published_timestamp().unwrap_or(0);
    let btimestamp = b.get_published_timestamp().unwrap_or(0);
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
  write!(f, r#"<?xml version="1.0" encoding="UTF-8"?>{}"#,quick_xml::se::to_string(&new_rss).unwrap_or(String::from("Failed")).replace("<content:encoded/>", ""));*/
}
