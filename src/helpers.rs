use std::fs::File;
use std::io::{Write};
use futures_util::StreamExt;

pub enum DownloadImageError {
  Reqwest(reqwest::Error),
  FileOpen(std::io::Error),
  FileWrite(std::io::Error)
}

impl std::fmt::Display for DownloadImageError {
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
     match self {
        DownloadImageError::Reqwest(error) => write!(f, "Error making request: {}", error),
        DownloadImageError::FileOpen(error) => write!(f, "Error opening file: {}", error),
        DownloadImageError::FileWrite(error) => write!(f, "Error writing file: {}", error)
     }
  }
}

pub enum DownloadImageOptions {
  Url(String),
  UrlAndOutputDir(String, String)
}

pub async fn download_image(params: DownloadImageOptions) -> Result<String, DownloadImageError> {
  let (url, out_dir) = match params {
    DownloadImageOptions::Url(url) => {
      (url, String::from("output/media/"))
    }, 
    DownloadImageOptions::UrlAndOutputDir(url, out_dir) => {
      (url, out_dir)
    }
  }; 
  let client = reqwest::Client::new();
  match client.get(&url).send().await {
    Ok(response) => {
      let file_name = url.replace("https://", "").replace("http://", "");
      let mut file = match File::create(format!("{}{}", out_dir, file_name)) {
        Ok(file) => file,
        Err(error) => {
          return Err(DownloadImageError::FileOpen(error));
        }
      };
      let mut stream = response.bytes_stream();
      while let Some(chunk) = stream.next().await {
        match chunk {
          Ok(chunk) => {
            match file.write(&chunk) {
              Ok(_) => {},
              Err(error) => {
                return Err(DownloadImageError::FileWrite(error))
              }
            }
          },
          Err(err) => {
            return Err(DownloadImageError::Reqwest(err));
          }
        }
      }
      log::info!("Finished downloading file: {file_name}");
      Ok(file_name.to_string())
    },
    Err(error) => {
      Err(DownloadImageError::Reqwest(error))
    }
  }
}
