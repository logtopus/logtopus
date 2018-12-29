mod constants;

use clap::{App, Arg};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let cli_matches = parse_cli();

    env_logger::init();

    logtopus::run(&cli_matches.value_of("config"))
}

fn parse_cli() -> clap::ArgMatches<'static> {
    App::new("logtopus server")
        .version(constants::VERSION)
        .author(constants::AUTHORS)
        .about(
            "Provides main logtopus server gateway communicating with the tentacles in a cluster.",
        )
        .arg(
            Arg::with_name("config")
                // .required(true)
                .short("c")
                .long("config")
                .value_name("FILE")
                .help("Sets the configuration file name")
                .takes_value(true),
        )
        .get_matches()
}
