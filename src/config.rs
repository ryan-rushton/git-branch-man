use std::path::PathBuf;

use color_eyre::eyre::Result;
use directories::ProjectDirs;
use lazy_static::lazy_static;
use serde::Deserialize;

const VERSION_MESSAGE: &str =
  concat!(env!("CARGO_PKG_VERSION"), "-", env!("VERGEN_GIT_DESCRIBE"), " (", env!("VERGEN_BUILD_DATE"), ")");

lazy_static! {
  pub static ref PROJECT_NAME: String = env!("CARGO_CRATE_NAME").to_uppercase().to_string();
  pub static ref DATA_FOLDER: Option<PathBuf> =
    std::env::var(format!("{}_DATA", PROJECT_NAME.clone())).ok().map(PathBuf::from);
  pub static ref CONFIG_FOLDER: Option<PathBuf> =
    std::env::var(format!("{}_CONFIG", PROJECT_NAME.clone())).ok().map(PathBuf::from);
  pub static ref LOG_ENV: String = format!("{}_LOGLEVEL", PROJECT_NAME.clone());
  pub static ref LOG_FILE: String = format!("{}.log", env!("CARGO_PKG_NAME"));
}

fn project_directory() -> Option<ProjectDirs> {
  ProjectDirs::from("com", "rrushton", env!("CARGO_PKG_NAME"))
}

#[derive(Clone, Debug, Deserialize, Default)]
pub struct AppConfig {
  #[serde(default)]
  pub _data_dir: PathBuf,
  #[serde(default)]
  pub _config_dir: PathBuf,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Config {
  #[serde(default, flatten)]
  pub config: AppConfig,
}

impl Config {
  pub fn new() -> Result<Self, config::ConfigError> {
    let data_dir = get_data_dir();
    let config_dir = get_config_dir();
    let mut builder = config::Config::builder()
      .set_default("_data_dir", data_dir.to_str().unwrap())?
      .set_default("_config_dir", config_dir.to_str().unwrap())?;

    let config_files = [
      ("config.json5", config::FileFormat::Json5),
      ("config.json", config::FileFormat::Json),
      ("config.yaml", config::FileFormat::Yaml),
      ("config.toml", config::FileFormat::Toml),
      ("config.ini", config::FileFormat::Ini),
    ];
    let mut found_config = false;
    for (file, format) in &config_files {
      builder = builder.add_source(config::File::from(config_dir.join(file)).format(*format).required(false));
      if config_dir.join(file).exists() {
        found_config = true
      }
    }
    if !found_config {
      // log::error!("No configuration file found. Application may not behave as expected");
    }

    let cfg: Self = builder.build()?.try_deserialize()?;

    Ok(cfg)
  }
}

pub fn get_data_dir() -> PathBuf {
  let directory = if let Some(s) = DATA_FOLDER.clone() {
    s
  } else if let Some(proj_dirs) = project_directory() {
    proj_dirs.data_local_dir().to_path_buf()
  } else {
    PathBuf::from(".").join(".data")
  };
  directory
}

pub fn get_config_dir() -> PathBuf {
  let directory = if let Some(s) = CONFIG_FOLDER.clone() {
    s
  } else if let Some(proj_dirs) = project_directory() {
    proj_dirs.config_local_dir().to_path_buf()
  } else {
    PathBuf::from(".").join(".config")
  };
  directory
}

pub fn version() -> String {
  let author = clap::crate_authors!();

  // let current_exe_path = PathBuf::from(clap::crate_name!()).display().to_string();
  let config_dir_path = get_config_dir().display().to_string();
  let data_dir_path = get_data_dir().display().to_string();
  let log_level = std::env::var(LOG_ENV.clone()).unwrap_or_else(|_| format!("{}=info", env!("CARGO_CRATE_NAME")));

  format!(
    "\
{VERSION_MESSAGE}

Authors: {author}
Config directory: {config_dir_path}
Data directory: {data_dir_path}
Log level: {log_level}"
  )
}
