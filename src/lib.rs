mod cfg;
mod log_merge;
mod server;
mod tentacle;

use crate::cfg::read_config;
use crate::server::start_server;
use std::error::Error;
use std::sync::Arc;

pub fn run<S: AsRef<str>>(maybe_settings: &Option<S>) -> Result<(), Box<dyn Error>> {
    let settings = Arc::new(read_config(maybe_settings).unwrap());

    let sys = actix::System::new("logtopus");

    start_server(settings.clone());

    sys.run();

    Ok(())
}
