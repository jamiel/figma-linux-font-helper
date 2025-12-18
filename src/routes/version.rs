use crate::config::Config;
use crate::server::Route;

use simple_server::{Method, Request, ResponseBuilder, ResponseResult};

fn handler(_: Request<Vec<u8>>, mut response: ResponseBuilder, _config: &Config) -> ResponseResult {
  let json = r#"{"version":23}"#;

  Ok(
    response
      .header("Access-Control-Allow-Origin", "https://www.figma.com")
      .header("Access-Control-Allow-Private-Network", "true")
      .header("Content-Type", "application/json")
      .body(json.as_bytes().to_vec())?,
  )
}

pub fn init() -> Route {
  Route {
    method: Method::GET,
    path: String::from("/figma/version"),
    handler: Box::new(handler),
  }
}
