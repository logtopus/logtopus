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

    pub fn stream_log<S: AsRef<str>>(
        &self,
        log: S,
    ) -> Box<dyn Stream<Item = String, Error = TentacleClientError>> {
        let tentacle = self.tentacles.first().unwrap();
        let log_encoded = quote(log, b"").unwrap();
        let url = format!("{}/api/v1/sources/{}/content", tentacle, log_encoded);
        let req = client::get(url)
            .header("User-Agent", "Actix-web")
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
        let lines = bytes.map(|b| String::from_utf8(b.to_vec()).unwrap());
        Box::new(lines)
    }
}
