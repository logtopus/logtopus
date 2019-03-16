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
    IllegalAliasError,
}

#[derive(Clone, PartialEq, Debug)]
pub struct TentacleInfo {
    pub name: String,
    pub host: String,
    pub port: i64,
    pub protocol: String,
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
    pub loglevel: Option<String>,
}

#[derive(Clone, Serialize, Debug, PartialEq)]
pub struct LogLine {
    pub timestamp: i64,
    pub message: String,
    pub loglevel: Option<String>,
    pub id: String,
    pub source: String,
}

pub struct TentacleClient {
    tentacles: Vec<TentacleInfo>,
}

impl TentacleClient {
    pub fn parse_tentacle(v: Value) -> Result<TentacleInfo, TentacleConfigError> {
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
                let name = table
                    .get("alias")
                    .map(|v| {
                        v.clone()
                            .into_str()
                            .map_err(|_| TentacleConfigError::IllegalAliasError)
                    })
                    .unwrap_or(Ok(host.clone()))?;
                Ok(TentacleInfo {
                    name,
                    host,
                    port,
                    protocol,
                })
            }
            Err(_e) => Err(TentacleConfigError::NoTableError),
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

    fn query_tentacle(
        &self,
        tentacle: TentacleInfo,
        id: String,
        from_ms: u64,
        loglevels: &Option<String>,
    ) -> LogStream {
        let id_encoded = quote(&id, b"").unwrap();
        let filter = loglevels
            .clone()
            .map(|f| format!("?from_ms={}&loglevels={}", from_ms, f))
            .unwrap_or(format!("?from_ms={}", from_ms));
        let url = format!(
            "{}/api/v1/sources/{}/content{}",
            tentacle.uri(),
            id_encoded,
            filter
        );
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
            .map(move |b| {
                let line = String::from_utf8(b.to_vec()).unwrap();
                let log_line: TentacleLogLine = serde_json::from_str(&line).unwrap();
                LogLine {
                    timestamp: log_line.timestamp,
                    message: log_line.message,
                    loglevel: log_line.loglevel,
                    id: id.clone(),
                    source: tentacle.name.clone(),
                }
            })
            .map_err(|_| LogStreamError::DefaultError);
        Box::new(lines)
    }

    pub fn stream_logs(
        &self,
        id: String,
        from_ms: u64,
        loglevels: &Option<String>,
    ) -> Box<dyn Stream<Item = LogLine, Error = TentacleClientError>> {
        let streams: Vec<LogStream> = self
            .tentacles
            .clone()
            .into_iter()
            .map(|t| self.query_tentacle(t.clone(), id.clone(), from_ms, loglevels))
            .collect();
        Box::new(LogMerge::new(streams).map_err(|_| TentacleClientError::ClientError))
    }
}
