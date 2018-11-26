// extern crate actix;
// extern crate actix_web;
extern crate bytes;
extern crate clap;
extern crate config;
extern crate env_logger;
extern crate futures;
extern crate futures_fs;
extern crate hyper;
extern crate tokio;
#[macro_use]
extern crate log;

use clap::{App, Arg, ArgMatches};
use hyper::service::service_fn_ok;
use hyper::{Body, Request, Response, Server};
//use std::collections::HashMap;
use futures::{Future, Stream};
use futures_fs::{FsPool, ReadOptions};
use std::fs::File;
use std::io::prelude::*;

pub mod cfg;
// mod server;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");
const AUTHORS: &'static str = env!("CARGO_PKG_AUTHORS");
const PHRASE: &str = "Hello, World!";

fn hello_world(_req: Request<Body>) -> Response<Body> {
    // our source file
    let fs = FsPool::default();
    let read = fs.read("Cargo.toml", ReadOptions::default().buffer_size(80));

    let what = read.map(|b| {
        let s = std::str::from_utf8(&b);
        let line = s.unwrap().to_owned();
        format!("line: {}\n", line)
    });

    Response::new(Body::wrap_stream(what))
}

fn main() -> std::io::Result<()> {
    // This is our socket address...
    let addr = ([127, 0, 0, 1], 3000).into();

    // A `Service` is needed for every connection, so this
    // creates one from our `hello_world` function.
    let new_svc = || {
        // service_fn_ok converts our function into a `Service`
        service_fn_ok(hello_world)
    };

    let server = Server::bind(&addr)
        .serve(new_svc)
        .map_err(|e| eprintln!("server error: {}", e));

    // Run this server for... forever!
    hyper::rt::run(server);

    Ok(())
}
