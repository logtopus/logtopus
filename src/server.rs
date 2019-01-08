extern crate actix;
extern crate actix_web;

use crate::tentacle::Tentacle;
use actix_web::HttpResponse;
use bytes::Bytes;
use config;
use futures::Stream;

pub fn start_server(settings: &config::Config) {
    let port = settings.get_int("http.bind.port").unwrap();
    let ip = settings.get_str("http.bind.ip").unwrap();
    let addr: std::net::SocketAddr = format!("{}:{}", ip, port).parse().unwrap();

    actix_web::server::new(move || {
        actix_web::App::new()
            // enable logger
            .middleware(actix_web::middleware::Logger::default())
            .prefix("/api/v1")
            .resource("/health", |r| r.get().f(|_| HttpResponse::Ok()))
            .resource("/logs/by-path/{path}", |r| r.get().with(stream_tentacle))
    })
    .bind(addr)
    .expect(&format!("Failed to bind to {}:{}", ip, port))
    .start();

    println!("Started http server: {:?}", addr);
}

fn stream_tentacle(log: actix_web::Path<String>) -> HttpResponse {
    let log_stream = Tentacle::stream_log(log.as_str());
    HttpResponse::Ok().streaming(
        log_stream
            .map(|s| Bytes::from(s))
            .map_err(|_| actix_web::error::PayloadError::Incomplete),
    )
}
