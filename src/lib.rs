mod cfg;
mod server;
pub mod tentacle;

use crate::cfg::read_config;
use crate::server::start_server;
use std::error::Error;

pub fn run<S: AsRef<str>>(maybe_settings: &Option<S>) -> Result<(), Box<dyn Error>> {
    let settings = read_config(maybe_settings).unwrap();

    let sys = actix::System::new("logtopus");

    start_server(&settings);

    sys.run();

    Ok(())
}
