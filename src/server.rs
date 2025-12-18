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
    let host = self.config.host.clone();
    let port = self.config.port.clone();

    info!("{:?}", &self.config);

    let serv = Arc::new(self);

    // Restart loop to handle thread pool panics from broken pipe errors
    loop {
      let s = serv.clone();

      let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        let serv_inner = s.clone();
        let server = simple_server::Server::new(move |request, mut response| {
          info!("Request received. {} {}", request.method(), request.uri());

          let s = serv_inner.as_ref();
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
        });

        server.listen(&host, &port);
      }));

      match result {
        Ok(_) => {
          // Server stopped cleanly
          warn!("Server stopped unexpectedly");
          break;
        }
        Err(e) => {
          // Check if this is a thread pool panic from broken pipe
          let is_thread_pool_panic = if let Some(s) = e.downcast_ref::<String>() {
            s.contains("Thread pool worker panicked")
              || s.contains("Broken pipe")
              || s.contains("BrokenPipe")
          } else if let Some(s) = e.downcast_ref::<&str>() {
            s.contains("Thread pool worker panicked")
              || s.contains("Broken pipe")
              || s.contains("BrokenPipe")
          } else {
            false
          };

          if is_thread_pool_panic {
            warn!("Client disconnected (broken pipe) - restarting server");
            // Brief pause before restart
            std::thread::sleep(std::time::Duration::from_millis(100));
            continue;
          } else {
            // Some other panic - log it and stop
            warn!("Server panicked with unexpected error - stopping");
            panic::resume_unwind(e);
          }
        }
      }
    }
  }
}

pub type Handler =
  Box<dyn Fn(Request<Vec<u8>>, ResponseBuilder, &Config) -> ResponseResult + 'static + Send + Sync>;

pub struct Route {
  pub method: Method,
  pub path: String,
  pub handler: Handler,
}
