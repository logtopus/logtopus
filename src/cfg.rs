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
