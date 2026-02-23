use std::{fmt, str::FromStr};

use clap::{Parser, builder::ArgPredicate};
use serde::{Deserialize, Serialize};
// ["trace", "debug", "info", "warn", "error", "none"]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, clap::ValueEnum, Serialize, Default)]
pub enum LogLevel {
    Trace,
    Debug,
    #[default]
    Info,
    Warn,
    Error,
    None,
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let str = match self {
            Self::Debug => "debug",
            Self::Error => "error",
            Self::Info => "info",
            Self::None => "none",
            Self::Trace => "trace",
            Self::Warn => "warn",
        };
        f.write_str(str)
    }
}

impl FromStr for LogLevel {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let lower = s.to_lowercase();
        match lower.as_str() {
            "trace" => Ok(Self::Trace),
            "debug" => Ok(Self::Debug),
            "info" => Ok(Self::Info),
            "warn" => Ok(Self::Warn),
            "error" => Ok(Self::Error),
            "none" => Ok(Self::None),
            _ => Err(s.to_owned()),
        }
    }
}

impl<'de> serde::Deserialize<'de> for LogLevel {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct PostVistor;
        impl<'de> serde::de::Visitor<'de> for PostVistor {
            type Value = LogLevel;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("expect log levels: trace, debug, info, warn, error, none")
            }
            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                v.parse::<Self::Value>()
                    .map_err(|err| serde::de::Error::custom(format!("invalid log level: {}", err)))
            }
        }
        deserializer.deserialize_str(PostVistor)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, clap::ValueEnum, Serialize, Default)]
pub enum LogOutput {
    #[default]
    STDOUT,
    STDERR,
    FILE,
}

impl FromStr for LogOutput {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let lower = s.to_lowercase();
        match lower.as_str() {
            "stdout" => Ok(Self::STDOUT),
            "stderr" => Ok(Self::STDERR),
            "file" => Ok(Self::FILE),
            _ => Err(s.to_owned()),
        }
    }
}

impl std::fmt::Display for LogOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let str = match self {
            Self::FILE => "file",
            Self::STDERR => "stderr",
            Self::STDOUT => "stdout",
        };
        f.write_str(str)
    }
}

impl<'de> serde::de::Deserialize<'de> for LogOutput {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct PostVistor;
        impl<'de> serde::de::Visitor<'de> for PostVistor {
            type Value = LogOutput;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("expect log output options: stdout, stderr, file")
            }
            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                v.parse::<Self::Value>()
                    .map_err(|err| serde::de::Error::custom(format!("invalid log output: {}", err)))
            }
        }
        deserializer.deserialize_str(PostVistor)
    }
}

#[derive(Debug, Parser, Deserialize, Serialize, Default)]
pub struct LoggerCli {
    #[arg(
        long,
        env,
        ignore_case = true,
        value_name = "LOG_LEVEL",
        help = "logger level to use",
        default_value = "info"
    )]
    pub log_level: LogLevel,
    #[arg(
        long,
        env,
        ignore_case = true,
        value_name = "LOG_OUTPUT",
        help = "log output",
        default_value_if("log_dir", ArgPredicate::IsPresent, "file"),
        default_value = "stdout"
    )]
    pub log_output: LogOutput,
    #[arg(
        long,
        env,
        ignore_case = true,
        value_name = "LOG_DIR",
        help = "log dir when output to file",
        required_if_eq("log_output", "file")
    )]
    pub log_dir: Option<std::path::PathBuf>,
}

impl std::fmt::Display for LoggerCli {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "log_level: {}, log_output: {}",
            self.log_level, self.log_output
        )?;
        if let Some(dir) = self.log_dir.as_ref() {
            write!(f, ", log_dir: {}", dir.display())?;
        }
        Ok(())
    }
}

#[derive(Parser, Serialize, Deserialize, Default)]
pub struct AppCliWithLogger<T: clap::Args + serde::Serialize + std::fmt::Display> {
    #[clap(flatten)]
    pub logger: LoggerCli,
    #[clap(flatten)]
    #[serde(flatten)]
    pub inner: T,
}

impl<T> std::fmt::Display for AppCliWithLogger<T>
where
    T: clap::Args + serde::Serialize + std::fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "logger: {}, config: {}", self.logger, self.inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_stdout() {
        let args = LoggerCli::try_parse_from(["test"]).unwrap();
        assert_eq!(args.log_output, LogOutput::STDOUT);
        assert!(args.log_dir.is_none());
    }

    #[test]
    fn logdir_implies_file() {
        let args = LoggerCli::try_parse_from(["test", "--log-dir", "logs"]).unwrap();
        assert_eq!(args.log_output, LogOutput::FILE);
        assert_eq!(
            args.log_dir.as_deref().map(|item| item.to_str()),
            Some(Some("logs"))
        );
    }

    #[test]
    fn user_can_override_to_stdout_even_with_logdir() {
        let args =
            LoggerCli::try_parse_from(["test", "--log-dir", "logs", "--log-output", "stdout"]);
        assert!(args.is_ok(), "parse should be ok");
        assert_eq!(args.unwrap().log_output, LogOutput::STDOUT);
    }

    #[test]
    fn file_requires_logdir() {
        let args = LoggerCli::try_parse_from(["test", "--log-output", "file"]);
        assert!(args.is_err(), "expect error cuz not log_dir provided");
    }

    #[test]
    fn case_insensitive_enum_works() {
        let args = LoggerCli::try_parse_from(["test", "--log-output", "StDoUT"]);
        assert!(args.is_ok(), "case intensive should be accpeted");
        let args = args.unwrap();
        assert_eq!(args.log_output, LogOutput::STDOUT);
    }
}
