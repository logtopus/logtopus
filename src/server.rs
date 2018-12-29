extern crate actix;
extern crate actix_web;

use actix::prelude::*;
use actix_web::HttpResponse;
use bytes::Bytes;
use config;
use futures::sync::mpsc::Sender;
use futures::Future;
use futures::Sink;
use futures::Stream;
use log::*;
use std::fs::File;
use std::io;
use std::io::{Error, ErrorKind};
use std::sync::Arc;
use std::sync::Mutex;
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
    let state = ServerState::new();

    actix_web::server::new(move || {
        actix_web::App::with_state(state.clone())
            // enable logger
            .middleware(actix_web::middleware::Logger::default())
            .resource("/health", |r| r.get().with(health))
            .resource("/log/{path}", |r| r.get().with(stream))
    })
    .bind(addr)
    .expect(&format!("Failed to bind to {}:{}", ip, port))
    .start();

    println!("Started http server: {:?}", addr);
}

fn health(_state: actix_web::State<ServerState>) -> &'static str {
    "OK"
}

fn stream(log: actix_web::Path<String>, state: actix_web::State<ServerState>) -> HttpResponse {
    let path = log.as_str().to_owned();
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
            StreamAdapterCommand::StreamFile(file, tx) => {
                let tfile = tokio::fs::File::from_std(file);
                let linereader = tokio::codec::FramedRead::new(
                    tfile,
                    tokio::codec::LinesCodec::new_with_max_length(4096),
                );
                let result = self.state.spawn_blocking(
                    linereader
                        .forward(
                            tx.sink_map_err(|e| Error::new(ErrorKind::InvalidData, e.to_string())),
                        )
                        .map(|_| debug!("Finished stream."))
                        .map_err(|e| {
                            error!("Stream error: {:?}", e);
                        }),
                );
                if let Err(e) = result {
                    error!("Failed to stream file: {}", e);
                }
                ()
            }
        }
    }
}
