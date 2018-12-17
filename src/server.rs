extern crate actix;
extern crate actix_web;

use actix::prelude::*;
use actix_web::{HttpRequest, HttpResponse};
use bytes::Bytes;
use config;
use futures::sync::mpsc::{Receiver, Sender};
use futures::Future;
use futures::Sink;
use futures::Stream;
use log::*;
use std::fs::File;
use std::io;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock;
use tokio::prelude::*;
use tokio::runtime::Runtime;

pub struct ServerState {
    rt: Arc<Mutex<Runtime>>,
}

impl Clone for ServerState {
    fn clone(&self) -> Self {
        ServerState {
            rt: self.rt.clone(),
        }
    }
}

impl ServerState {
    pub fn new() -> ServerState {
        ServerState {
            rt: Arc::new(Mutex::new(Runtime::new().unwrap())),
        }
    }

    pub fn spawn_blocking(
        &self,
        f: impl Future<Item = (), Error = ()> + Send + 'static,
    ) -> Result<(), io::Error> {
        let mutex = self.rt.lock();
        match mutex {
            Ok(mut rt) => {
                (*rt).spawn(f);
                Ok(())
            }
            Err(e) => Err(io::Error::new(io::ErrorKind::Other, e.to_string())),
        }
    }
}

pub fn start_server(settings: &config::Config) {
    let port = settings.get_int("http.bind.port").unwrap();
    let ip = settings.get_str("http.bind.ip").unwrap();
    let addr: std::net::SocketAddr = format!("{}:{}", ip, port).parse().unwrap();
    let runtime = Runtime::new().unwrap();
    let state = ServerState::new();

    actix_web::server::new(move || {
        actix_web::App::with_state(state.clone())
            // enable logger
            .middleware(actix_web::middleware::Logger::default())
            .resource("/", |r| r.get().with(index))
            .resource("/health", |r| r.get().with(health))
            .resource("/log/{path}", |r| r.get().with(stream))
    })
    .bind(addr)
    .expect(&format!("Failed to bind to {}:{}", ip, port))
    .start();

    println!("Started http server: {:?}", addr);
}

fn index(_state: actix_web::State<ServerState>) -> &'static str {
    "Hello world!\nBye, bye world!"
}

fn health(_state: actix_web::State<ServerState>) -> &'static str {
    "OK"
}

fn stream(log: actix_web::Path<String>, state: actix_web::State<ServerState>) -> HttpResponse {
    let path = log.as_str().to_owned();
    // let stream = stream_log(ret).map(|s| Bytes::from(s));
    let file = File::open(path).unwrap();
    let (tx, rx_body) = futures::sync::mpsc::channel(1024 * 1024);
    let streamer = StreamAdapter::new(state.clone()).start();
    let request = streamer.send(StreamAdapterCommand::StreamFile(file, tx));

    actix::spawn(request.map_err(|e| println!("Streaming Actor has probably died: {}", e)));
    HttpResponse::Ok().streaming(
        rx_body
            .map(|s| Bytes::from(s))
            .map_err(|_| actix_web::error::PayloadError::Incomplete),
    )
}

// pub fn stream_log<T: AsRef<std::path::Path>>(path: T) -> Receiver<String> {
//     // our source file
//     // let file = format!("{}", path);
//     // let fs = FsPool::default();
//     // let read = fs.read(file, ReadOptions::default().buffer_size(80));

//     // let tfile = tokio::fs::File::from_std(file);
//     // TODO: What is a reasonable line length limit?
//     // let linereader =
//     // tokio::codec::FramedRead::new(tfile, tokio::codec::LinesCodec::new_with_max_length(512));

//     // let what = read.map(|b| {
//     //     let s = std::str::from_utf8(&b);
//     //     s.unwrap().to_owned()
//     // });

//     // what.into_inner()
//     let (tx, rx_body) = futures::sync::mpsc::channel(1024 * 1024);
//     let streamer = StreamAdapter.start();
//     let request = streamer.send(StreamAdapterCommand::StreamFile(file, tx));

//     actix::spawn(request.map_err(|e| println!("Streaming Actor has probably died: {}", e)));
//     // linereader.map(|s| Bytes::from(s))
//     rx_body
// }

#[derive(Message)]
pub enum StreamAdapterCommand {
    StreamFile(File, Sender<String>),
}

struct StreamAdapter {
    state: ServerState,
}

impl StreamAdapter {
    fn new(state: ServerState) -> StreamAdapter {
        StreamAdapter { state: state }
    }
}

impl Actor for StreamAdapter {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        debug!("Started.");
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        debug!("Stopped.");
    }
}

impl Handler<StreamAdapterCommand> for StreamAdapter {
    type Result = ();

    fn handle(&mut self, msg: StreamAdapterCommand, _: &mut Context<Self>) -> Self::Result {
        debug!("Handle");
        match msg {
            StreamAdapterCommand::StreamFile(file, mut tx) => {
                let tfile = tokio::fs::File::from_std(file);
                let linereader = tokio::codec::FramedRead::new(
                    tfile,
                    tokio::codec::LinesCodec::new_with_max_length(4096),
                );
                self.state.spawn_blocking(
                    linereader
                        .for_each(move |s| match tx.start_send(s + "\n") {
                            Ok(_) => Ok(()),
                            Err(e) => {
                                error!("{}", e.to_string());
                                Err(io::Error::new(io::ErrorKind::Other, e.to_string()))
                            }
                        })
                        .map_err(|e| {
                            error!("Stream error: {:?}", e);
                        }),
                );
                ()
            }
        }
    }
}
