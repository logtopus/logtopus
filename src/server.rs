extern crate actix;
extern crate actix_web;

use actix_web::{HttpRequest, HttpResponse};
use bytes::Bytes;
use config;
use futures::{Future, Stream};
use futures_fs::{FsPool, FsReadStream, ReadOptions};
use std::fmt::Display;
use std::fs::File;

pub fn start_server(settings: &config::Config) {
    let port = settings.get_int("http.bind.port").unwrap();
    let ip = settings.get_str("http.bind.ip").unwrap();
    let addr: std::net::SocketAddr = format!("{}:{}", ip, port).parse().unwrap();

    actix_web::server::new(|| {
        actix_web::App::new()
            // enable logger
            .middleware(actix_web::middleware::Logger::default())
            .resource("/", |r| r.f(index))
            .resource("/health", |r| r.get().f(health))
            .resource("/log/{path}", |r| r.get().with(stream))
    })
    .bind(addr)
    .expect(&format!("Failed to bind to {}:{}", ip, port))
    .start();

    println!("Started http server: {:?}", addr);
}

fn index(_req: &HttpRequest) -> &'static str {
    "Hello world!\nBye, bye world!"
}

fn health(_req: &HttpRequest) -> &'static str {
    "OK"
}

fn stream(log: actix_web::Path<String>) -> HttpResponse {
    let ret = log.as_str().to_owned();
    let stream = stream_log(ret);
    HttpResponse::Ok().streaming(stream)
}

pub fn stream_log<T: AsRef<std::path::Path>>(
    path: T,
) -> Stream<Item = String, Error = std::io::Error> {
    // our source file
    // let file = format!("{}", path);
    // let fs = FsPool::default();
    // let read = fs.read(file, ReadOptions::default().buffer_size(80));

    let mut file = File::open(path).unwrap();
    let mut tfile = tokio::fs::File::from_std(file);
    let linereader =
        tokio::codec::FramedRead::new(tfile, tokio::codec::LinesCodec::new_with_max_length(2048));

    // let what = read.map(|b| {
    //     let s = std::str::from_utf8(&b);
    //     s.unwrap().to_owned()
    // });

    // what.into_inner()
    linereader
}
