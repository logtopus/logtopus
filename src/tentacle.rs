use crate::log_merge::{LogMerge, LogStream, LogStreamError};
use actix_web::{client, HttpMessage};
use config::{Config, Value};
use futures::{Future, Stream};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use urlparse::quote;

const DEFAULT_PORT: i64 = 8080;
const DEFAULT_PROTOCOL: &'static str = "http";

#[derive(Debug)]
pub enum TentacleClientError {
    ClientError,
}

#[derive(Debug)]
pub enum TentacleConfigError {
    NoTableError,
    NoHostSpecified,
    IllegalHostError,
    IllegalPortError,
    IllegalProtocolError,
}

pub struct TentacleInfo {
    host: String,
    port: i64,
    protocol: String,
}

impl TentacleInfo {
    pub fn uri(&self) -> String {
        format!("{}://{}:{}", self.protocol, self.host, self.port)
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct TentacleLogLine {
    pub timestamp: i64,
    pub message: String,
}

pub struct TentacleClient {
    tentacles: Vec<TentacleInfo>,
}

impl TentacleClient {
    fn parse_tentacle(v: Value) -> Result<TentacleInfo, TentacleConfigError> {
        match v.into_table() {
            Ok(table) => {
                let host = table
                    .get("host")
                    .cloned()
                    .ok_or(TentacleConfigError::NoHostSpecified)?
                    .into_str()
                    .map_err(|_| TentacleConfigError::IllegalHostError)?;
                let port = table
                    .get("port")
                    .map(|v| {
                        v.clone()
                            .into_int()
                            .map_err(|_| TentacleConfigError::IllegalPortError)
                    })
                    .unwrap_or(Ok(DEFAULT_PORT))?;
                let protocol = table
                    .get("protocol")
                    .map(|v| {
                        v.clone()
                            .into_str()
                            .map_err(|_| TentacleConfigError::IllegalProtocolError)
                    })
                    .unwrap_or(Ok(String::from(DEFAULT_PROTOCOL)))?;
                Ok(TentacleInfo {
                    host,
                    port,
                    protocol,
                })
            }
            Err(e) => Err(TentacleConfigError::NoTableError),
        }
    }

    pub fn from_settings(settings: Arc<Config>) -> Result<TentacleClient, TentacleConfigError> {
        let tentacles: Result<Vec<TentacleInfo>, TentacleConfigError> = settings
            .get_array("tentacles")
            .unwrap() // we unwrap here as there is always an empty array defined in default config
            .into_iter()
            .map(|v| TentacleClient::parse_tentacle(v))
            .collect();
        tentacles.map(|infos| TentacleClient { tentacles: infos })
    }

    fn query_tentacle(&self, tentacle: String, id: &String) -> LogStream {
        let id_encoded = quote(id, b"").unwrap();
        let url = format!("{}/api/v1/sources/{}/content", tentacle, id_encoded);
        let req = client::get(url)
            .header("User-Agent", "logtopus")
            .header("Accept", "application/json")
            .finish()
            .unwrap()
            .send()
            .map_err(|_| TentacleClientError::ClientError);
        let bytes = req
            .map(|response| {
                response
                    .payload()
                    .map_err(|_| TentacleClientError::ClientError)
            })
            .flatten_stream();
        let lines = bytes
            .map(|b| {
                let line = String::from_utf8(b.to_vec()).unwrap();
                serde_json::from_str(&line).unwrap()
            })
            .map_err(|_| LogStreamError::DefaultError);
        Box::new(lines)
    }

    pub fn stream_logs(
        &self,
        id: &String,
    ) -> Box<dyn Stream<Item = TentacleLogLine, Error = TentacleClientError>> {
        let streams: Vec<LogStream> = self
            .tentacles
            .as_slice()
            .into_iter()
            .map(|t| t.uri())
            .into_iter()
            .map(|t| self.query_tentacle(t, id))
            .collect();
        Box::new(LogMerge::new(streams).map_err(|_| TentacleClientError::ClientError))
    }
}
