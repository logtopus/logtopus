extern crate actix;
extern crate actix_web;

use crate::tentacle::Tentacle;
use actix_web::{HttpResponse, State};
use bytes::BufMut;
use bytes::Bytes;
use config;
use config::Config;
use futures::Stream;
use std::str::FromStr;
use std::sync::Arc;

pub fn start_server(settings: Arc<Config>) {
    let port = settings.get_int("http.bind.port").unwrap();
    let ip = settings.get_str("http.bind.ip").unwrap();
    let addr: std::net::SocketAddr = format!("{}:{}", ip, port).parse().unwrap();
    let state_factory = ServerStateFactory::from_settings(settings);

    actix_web::server::new(move || {
        actix_web::App::with_state(state_factory.create_state())
            // enable logger
            .middleware(actix_web::middleware::Logger::default())
            .prefix("/api/v1")
            .resource("/health", |r| r.get().f(|_| HttpResponse::Ok()))
            .resource("/sources/{id}/content", |r| r.get().with(stream_tentacle))
    })
    .bind(addr)
    .expect(&format!("Failed to bind to {}:{}", ip, port))
    .start();

    println!("Started http server: {:?}", addr);
}

fn stream_tentacle(id: actix_web::Path<String>, state: State<Tentacle>) -> HttpResponse {
    let log_stream = state.stream_logs(&String::from_str(id.as_str()).unwrap());
    HttpResponse::Ok().streaming(
        log_stream
            .map(move |log_line| {
                let mut json = serde_json::to_vec(&log_line).unwrap();
                json.put_u8('\n' as u8);
                Bytes::from(json)
            })
            .map_err(|_| actix_web::error::PayloadError::Incomplete),
    )
}

struct ServerStateFactory {
    settings: Arc<Config>,
}

impl Clone for ServerStateFactory {
    fn clone(&self) -> ServerStateFactory {
        ServerStateFactory {
            settings: self.settings.clone(),
        }
    }
}

impl ServerStateFactory {
    fn from_settings(settings: Arc<Config>) -> ServerStateFactory {
        ServerStateFactory { settings: settings }
    }

    fn create_state(&self) -> Tentacle {
        Tentacle::from_settings(self.settings.clone())
    }
}
