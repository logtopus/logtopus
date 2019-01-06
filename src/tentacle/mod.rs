use actix_web::{client, HttpMessage};
use futures::{Future, Stream};
use urlparse::quote;

#[derive(Debug)]
pub enum TentacleClientError {
    ClientError,
}

pub struct Tentacle {}

impl Tentacle {
    pub fn stream_log<S: AsRef<str>>(
        log: S,
    ) -> Box<dyn Stream<Item = String, Error = TentacleClientError>> {
        let log_encoded = quote(log, b"").unwrap();
        let url = format!(
            "http://localhost:8080/api/v1/sources/{}/content",
            log_encoded,
        );
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
