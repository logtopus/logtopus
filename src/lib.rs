mod cfg;
mod server;

use crate::cfg::read_config;
use crate::server::start_server;
use std::error::Error;

pub fn run<S: AsRef<str>>(maybeSettings: &Option<S>) -> Result<(), Box<dyn Error>> {
    let settings = read_config(maybeSettings).unwrap();

    let sys = actix::System::new("logtopus");

    start_server(&settings);

    sys.run();

    Ok(())
}
