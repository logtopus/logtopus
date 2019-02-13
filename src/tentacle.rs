use crate::log_merge::{LogMerge, LogMergeError, LogStream};
use actix_web::{client, HttpMessage};
use config::Config;
use futures::{Future, Stream};
use std::sync::Arc;
use urlparse::quote;

#[derive(Debug)]
pub enum TentacleClientError {
    ClientError,
}

pub struct Tentacle {
    tentacles: Vec<String>,
}

impl Tentacle {
    pub fn from_settings(settings: Arc<Config>) -> Tentacle {
        let tentacles = settings
            .get_array("tentacles")
            .unwrap()
            .into_iter()
            .map(|v| v.into_str().unwrap())
            .collect();
        Tentacle {
            tentacles: tentacles,
        }
    }

    fn query_tentacle(&self, tentacle: String, id: &String) -> LogStream {
        let id_encoded = quote(id, b"").unwrap();
        let url = format!("{}/api/v1/sources/{}/content", tentacle, id_encoded);
        let req = client::get(url)
            .header("User-Agent", "logtopus")
            .header("Accept", "text/plain")
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
            .map(|b| String::from_utf8(b.to_vec()).unwrap())
            .map_err(|_| LogMergeError::DefaultError);
        Box::new(lines)
    }

    pub fn stream_logs(
        &self,
        id: &String,
    ) -> Box<dyn Stream<Item = String, Error = TentacleClientError>> {
        let streams: Vec<LogStream> = self
            .tentacles
            .clone()
            .into_iter()
            .map(|t| self.query_tentacle(t, id))
            .collect();
        Box::new(LogMerge::new(streams).map_err(|_| TentacleClientError::ClientError))
    }
}
