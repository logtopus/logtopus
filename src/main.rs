extern crate actix;
extern crate actix_web;
extern crate bytes;
extern crate clap;
extern crate config;
extern crate env_logger;
#[macro_use]
extern crate log;

use clap::{App, Arg, ArgMatches};
//use std::collections::HashMap;

pub mod cfg;
mod server;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");
const AUTHORS: &'static str = env!("CARGO_PKG_AUTHORS");

fn main() {
    let cli_matches = parse_cli();

    init_log(&cli_matches);

    let maybe_filename = cli_matches.value_of("config");

    let settings = match cfg::read_config(&maybe_filename) {
        Ok(config) => config,
        Err(msg) => {
            println!("Error: {}", msg);
            std::process::exit(1)
        }
    };

    let sys = actix::System::new("root");

    server::start_server(&settings);

    //    println!("\nConfiguration\n\n{:?} \n\n-----------",
    //             settings.try_into::<HashMap<String, config::Value>>().unwrap());

    sys.run();
}

fn init_log(matches: &ArgMatches) {
    let loglevel = match matches.occurrences_of("v") {
        0 => "error",
        1 => "warn",
        2 => "info",
        3 => "debug",
        _ => "trace",
    };

    let loglevel = match matches.value_of("module") {
        Some(module) => {
            let mut module_loglevel = String::from(module);
            module_loglevel.push_str("=");
            module_loglevel.push_str(loglevel);
            module_loglevel
        }
        _ => String::from(loglevel),
    };

    std::env::set_var("RUST_LOG", &loglevel);
    env_logger::init();
    debug!("Setting log level to {}", &loglevel);
}

fn parse_cli() -> clap::ArgMatches<'static> {
    App::new("Logtopus server")
        .version(VERSION)
        .author(AUTHORS)
        .about("Main logtopus server")
        .arg(
            Arg::with_name("config")
            // .required(true)
            .short("c")
            .long("config")
            .value_name("FILE")
            .help("Sets the configuration file name")
            .takes_value(true),
        ).arg(
            Arg::with_name("module")
                .short("m")
                .long("module")
                .takes_value(true)
                .help("Sets the optional name of the module for which to set the verbosity level"),
        ).arg(
            Arg::with_name("v")
                .short("v")
                .multiple(true)
                .help("Sets the level of verbosity via loglevel (error, warn, debug and trace)"),
        ).get_matches()
}

#[cfg(test)]
mod tests {
    use cfg;

    #[test]
    fn test_read_config() {
        let settings = cfg::read_config(&Some("tests/testconf.yml")).unwrap();
        assert_eq!(12345, settings.get_int("http.bind.port").unwrap());
        assert_eq!("127.0.0.1", settings.get_str("http.bind.ip").unwrap());
    }
}
