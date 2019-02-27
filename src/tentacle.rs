use crate::log_merge::{LogMerge, LogStream, LogStreamError};
use actix_web::{client, HttpMessage};
use config::Config;
use futures::{Future, Stream};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use urlparse::quote;

#[derive(Debug)]
pub enum TentacleClientError {
    ClientError,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct TentacleLogLine {
    pub timestamp: i64,
    pub message: String,
}

pub struct TentacleClient {
    tentacles: Vec<String>,
}

impl TentacleClient {
    pub fn from_settings(settings: Arc<Config>) -> TentacleClient {
        let tentacles = settings
            .get_array("tentacles")
            .unwrap()
            .into_iter()
            .map(|v| v.into_str().unwrap())
            .collect();
        TentacleClient {
            tentacles: tentacles,
        }
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
            .clone()
            .into_iter()
            .map(|t| self.query_tentacle(t, id))
            .collect();
        Box::new(LogMerge::new(streams).map_err(|_| TentacleClientError::ClientError))
    }
}
