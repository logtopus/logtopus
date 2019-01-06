use futures::stream::Stream;
use log::*;
use logtopus::tentacle::Tentacle;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    info!("Tentacle test client...");

    let mut sys = actix::System::new("logtopus");

    let log_file = "/var/log/syslog";
    let log = Tentacle::stream_log(log_file);
    let printer = log.for_each(|line| {
        println!("{}", line);
        Ok(())
    });
    match sys.block_on(printer) {
        Ok(_) => info!("Request done!"),
        Err(e) => error!("Request failed: {:?}", e),
    }

    Ok(())
}
