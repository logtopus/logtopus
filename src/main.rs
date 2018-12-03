// extern crate actix;
// extern crate actix_web;
extern crate bytes;
extern crate clap;
extern crate config;
extern crate env_logger;
extern crate futures;
extern crate futures_fs;
extern crate http;
extern crate hyper;
extern crate tokio;
#[macro_use]
extern crate log;
extern crate gotham;
extern crate serde;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate gotham_derive;

use clap::{App, Arg, ArgMatches};
use http::Response;
use hyper::Body;
//use std::collections::HashMap;
use futures::{Future, Stream};
use futures_fs::{FsPool, ReadOptions};
use gotham::router::builder::*;
use gotham::router::Router;
use gotham::state::{FromState, State};

pub mod cfg;
// mod server;

fn stream_log(state: State) -> (State, Response<Body>) {
    // our source file
    let file = {
        let path = PathExtractor::borrow_from(&state);
        path.path.to_owned()
    };
    let fs = FsPool::default();
    let read = fs.read(file, ReadOptions::default().buffer_size(80));

    let what = read.map(|b| {
        let s = std::str::from_utf8(&b);
        let line = s.unwrap().to_owned();
        format!("line: {}\n", line)
    });

    (state, Response::new(Body::wrap_stream(what)))
}

#[derive(Deserialize, StateData, StaticResponseExtender)]
struct PathExtractor {
    path: String,
}

fn router() -> Router {
    build_simple_router(|route| {
        route
            // Note the use of :name variable in the path defined here. The router will map the
            // second (and last) segment of this path to the field `name` when extracting data.
            .get("/log/:path")
            // This tells the Router that for requests which match this route that path extraction
            // should be invoked storing the result in a `PathExtractor` instance.
            .with_path_extractor::<PathExtractor>()
            .to(stream_log);
    })
}

fn start_gotham() {
    let addr = "127.0.0.1:3001";
    gotham::start_with_num_threads(addr, router(), 4);
}

fn main() -> std::io::Result<()> {
    start_gotham();
    Ok(())
}
