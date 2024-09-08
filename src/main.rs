use clap::Parser;
use color_eyre::eyre::Result;

use crate::{app::App, cli::Cli};

pub mod action;
pub mod app;
pub mod cli;
pub mod components;
pub mod config;
pub mod error;
pub mod errors;
pub mod git;
pub mod logging;
pub mod mode;
pub mod tui;

async fn tokio_main() -> Result<()> {
  logging::init()?;
  errors::init()?;

  Cli::parse();
  let mut app = App::new()?;
  app.run().await?;

  Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
  if let Err(e) = tokio_main().await {
    eprintln!("{} error: Something went wrong", env!("CARGO_PKG_NAME"));
    Err(e)
  } else {
    Ok(())
  }
}
