extern crate libfonthelper;
extern crate log;
extern crate simple_server;

use super::config::Config;
use super::routes::*;
use log::{info, warn};
use simple_server::{Method, Request, ResponseBuilder, ResponseResult, StatusCode};
use std::panic;
use std::sync::Arc;

pub struct Server {
  config: Config,
  routes: Box<Vec<Route>>,
}

impl Server {
  pub fn new(config: Config) -> Self {
    Server {
      config,
      routes: Box::new(vec![]),
    }
  }

  pub fn add_route(mut self, route: Route) -> Self {
    self.routes.push(route);
    self
  }

  pub fn start(self) {
    // Set up panic hook to gracefully handle broken pipe errors
    let default_panic = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
      // Check if this is a broken pipe error and just log it
      let payload = panic_info.payload();
      if let Some(s) = payload.downcast_ref::<String>() {
        if s.contains("Broken pipe") || s.contains("BrokenPipe") {
          warn!("Client disconnected (broken pipe) - this is normal");
          return;
        }
      } else if let Some(s) = payload.downcast_ref::<&str>() {
        if s.contains("Broken pipe") || s.contains("BrokenPipe") {
          warn!("Client disconnected (broken pipe) - this is normal");
          return;
        }
      }
      // For other panics, use the default handler
      default_panic(panic_info);
    }));

    let serv = Arc::new(self);
    let s = serv.clone();

    let server = simple_server::Server::new(move |request, mut response| {
      info!("Request received. {} {}", request.method(), request.uri());

      // Catch panics to prevent broken pipe errors from crashing the server
      let result = panic::catch_unwind(panic::AssertUnwindSafe(|| -> ResponseResult {
        let s = serv.as_ref();
        let routes = Arc::new(s.routes.as_ref());

        if request.method() == Method::OPTIONS {
          return Ok(
            response
              .status(StatusCode::NO_CONTENT)
              .header("Access-Control-Allow-Origin", "https://www.figma.com")
              .header("Access-Control-Allow-Private-Network", "true")
              .header("Content-Type", "application/octet-stream")
              .body("".as_bytes().to_vec())?,
          );
        }

        for route in *routes {
          if route.method == request.method() && route.path == request.uri().path() {
            let handler = &(route.handler).as_ref();
            return handler(request, response, &s.config);
          }
        }

        any::handler(request, response)
      }));

      match result {
        Ok(r) => r,
        Err(e) => {
          // Log the panic but don't crash - this is usually a broken pipe
          if let Some(s) = e.downcast_ref::<&str>() {
            warn!("Request handler panicked: {}", s);
          } else if let Some(s) = e.downcast_ref::<String>() {
            warn!("Request handler panicked: {}", s);
          } else {
            warn!("Request handler panicked with unknown error");
          }
          // Can't send a response since the handler already consumed the ResponseBuilder
          // and the client likely disconnected anyway. Return a dummy error.
          Err(simple_server::Error::from(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Handler panicked",
          )))
        }
      }
    });

    info!("{:?}", &s.config);

    server.listen(&s.config.host, &s.config.port);
  }
}

pub type Handler =
  Box<dyn Fn(Request<Vec<u8>>, ResponseBuilder, &Config) -> ResponseResult + 'static + Send + Sync>;

pub struct Route {
  pub method: Method,
  pub path: String,
  pub handler: Handler,
}
