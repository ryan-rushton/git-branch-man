use clap::Parser;

use crate::config::version;

#[derive(Parser, Debug)]
#[command(author, version = version(), about)]
pub struct Cli {}
