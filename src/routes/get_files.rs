use crate::config::Config;
use crate::server::Route;

use libfonthelper::FontsHelper;
use simple_server::{Method, Request, ResponseBuilder, ResponseResult};
use std::fs;
use std::time::SystemTime;

fn handler(_: Request<Vec<u8>>, mut response: ResponseBuilder, config: &Config) -> ResponseResult {
  let fonts = FontsHelper::new(&config.app.font_dirs);

  let mut json = "{\"version\": 23,\"fontFiles\":".to_string();
  json.push_str("{");

  for font in fonts {
    if font.entries.len() > 0 {
      // Get modification timestamp for this font file
      let modified_at = fs::metadata(&font.path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);

      json.push_str(&format!("\"{}\":[", font.path));
      for num in 0..font.entries.len() {
        json.push_str("{");
        json.push_str(&format!(
          "\"postscript\": \"{}\",",
          font.entries[num].postscript
        ));
        json.push_str(&format!("\"family\": \"{}\",", font.entries[num].family));
        json.push_str(&format!("\"id\": \"{}\",", font.entries[num].id));
        json.push_str(&format!("\"style\": \"{}\",", font.entries[num].style));
        json.push_str(&format!("\"weight\": {},", font.entries[num].weight));
        json.push_str(&format!("\"stretch\": {},", font.entries[num].stretch));
        json.push_str(&format!("\"italic\": {},", font.entries[num].italic));
        json.push_str(&format!("\"modified_at\": {},", modified_at));
        json.push_str("\"user_installed\": true");
        json.push_str("},");
      }
      json.pop();
      json.push_str("],");
    }
  }

  json.pop();
  json.push_str("}}");

  Ok(
    response
      .header("Access-Control-Allow-Origin", "https://www.figma.com")
      .header("Access-Control-Allow-Private-Network", "true")
      .header("Content-Type", "application/json")
      .header("Content-Length", json.bytes().len())
      .body(json.as_bytes().to_vec())?,
  )
}

pub fn init() -> Route {
  Route {
    method: Method::GET,
    path: String::from("/figma/font-files"),
    handler: Box::new(handler),
  }
}
