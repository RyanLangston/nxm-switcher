// Work on this
// TODO: DO it

use clap::{Parser, Subcommand, command};
use serde::Deserialize;
use std::process::Command;

const MIME_TYPES: &[&str] = &["x-scheme-handler/nxm", "x-scheme-handler/nxm-protocol"];

#[derive(Deserialize)]
struct Handler {
    name: String,
    // TODO: Make this its own type
    file: String,
}

/// CLI Arguments that it'll accept
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Status,
    Select,
}

fn main() {
    println!("Placeholder")
}
