mod logger;
pub use logger::*;
mod protocol;
pub use protocol::*;
mod ip_family;
pub use ip_family::*;
use serde::de::Error;

use std::fmt::Debug;

use figment::{
    Figment,
    providers::{Format, Json, Serialized, Toml, Yaml},
};

use clap::Parser;

#[derive(Debug, Parser, serde::Serialize, serde::Deserialize, Default)]
pub struct AppCli<T: clap::Args + serde::Serialize + std::fmt::Display> {
    #[arg(long, short = 'c', value_name = "CONFIG", help = "config file")]
    pub config: Option<std::path::PathBuf>,
    #[clap(flatten)]
    #[serde(flatten)]
    pub inner: T,
}

impl<T> std::fmt::Display for AppCli<T>
where
    T: clap::Args + serde::Serialize + std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(config) = self.config.as_ref() {
            write!(f, "config_file: {}, ", config.display())?;
        }
        write!(f, "config: {}", self.inner)
    }
}

struct Ini;

impl figment::providers::Format for Ini {
    type Error = serde_ini::de::Error;

    const NAME: &'static str = "INI";

    fn from_str<'de, T: serde::de::DeserializeOwned>(string: &'de str) -> Result<T, Self::Error> {
        serde_ini::de::from_str(string)
    }
}

impl<T> AppCli<T>
where
    T: clap::Args + serde::Serialize + for<'a> serde::Deserialize<'a> + std::fmt::Display + Default,
{
    pub fn merge(self) -> Result<Self, figment::Error> {
        let config_file = self.config.clone();

        let mut result = Figment::from(Serialized::defaults(self));
        if let Some(config_file) = config_file.as_ref() {
            if !config_file.is_file()
                || !std::fs::exists(&config_file).map_err(|err| {
                    figment::Error::custom(format!(
                        "error check config file {} exists, err: {}",
                        config_file.display(),
                        err
                    ))
                })?
            {
                return Err(figment::Error::custom(format!(
                    "config file {} not found",
                    config_file.display()
                )));
            }

            let extension = config_file
                .extension()
                .map(|str| str.to_str().unwrap_or(""))
                .unwrap_or("");
            let provider = match extension {
                "toml" => Figment::from(Toml::file(config_file)),
                "json" => Figment::from(Json::file(config_file)),
                // "ini" | "conf" => Figment::from(Ini::file(config_file)), // ini is problematic, ee https://github.com/arcnmx/serde-ini/issues/6
                _ => Figment::from(Yaml::file(config_file)),
            };
            result = result.merge(provider);
        }
        Ok(result.extract()?)
    }
}
