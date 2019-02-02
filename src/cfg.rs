use config;
use config::ConfigError;
use std::path::Path;

pub fn read_config<S: AsRef<str>>(
    maybe_filename: &Option<S>,
) -> Result<config::Config, config::ConfigError> {
    let mut settings = config::Config::new();

    let defaults = include_bytes!("default_config.yml");
    settings.merge(config::File::from_str(
        String::from_utf8_lossy(defaults).as_ref(),
        config::FileFormat::Yaml,
    ))?;

    match maybe_filename {
        Some(filename_ref) => {
            let filename = filename_ref.as_ref();
            if !Path::new(filename).exists() {
                return Err(ConfigError::Message(format!(
                    "Configuration file {} does not exist",
                    filename
                )));
            } else {
                settings.merge(config::File::with_name(&filename))?
            }
        }
        None => &settings,
    };
    settings.merge(config::Environment::with_prefix("app"))?;

    Ok(settings)
}

#[cfg(test)]
mod tests {
    use crate::cfg;

    #[test]
    fn test_read_config() {
        let settings = cfg::read_config(&Some("tests/test.yml")).unwrap();

        assert_eq!(28081, settings.get_int("http.bind.port").unwrap());
        assert_eq!("127.0.0.1", settings.get_str("http.bind.ip").unwrap());

        let tentacles: Vec<String> = settings
            .get_array("tentacles")
            .unwrap()
            .into_iter()
            .map(|v| v.into_str().unwrap())
            .collect();

        assert_eq!(
            vec!["http://server-1:8080", "http://server-2:8080"],
            tentacles
        );
    }

    #[test]
    fn test_read_default_config() {
        let settings = cfg::read_config::<String>(&None).unwrap();

        assert_eq!(8081, settings.get_int("http.bind.port").unwrap());
        assert_eq!("127.0.0.1", settings.get_str("http.bind.ip").unwrap());

        let tentacles: Vec<String> = settings
            .get_array("tentacles")
            .unwrap()
            .into_iter()
            .map(|v| v.into_str().unwrap())
            .collect();

        assert!(tentacles.is_empty());
    }
}
