// Work on this
// TODO: DO it

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, command};
use serde::Deserialize;
use std::process::Command;

/// The MIME Types
const MIME_TYPES: &[&str] = &["x-scheme-handler/nxm", "x-scheme-handler/nxm-protocol"];

/// Handles config related things
#[derive(Deserialize)]
struct Handler {
    /// Name of the handler to be displayed to the user
    name: String,
    // TODO: Make this its own type
    /// Path to the .desktop file
    file: String,
}

/// CLI Arguments that it'll accept
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

/// Possible Commands the user can use
#[derive(Subcommand)]
enum Commands {
    /// Status of the mime
    Status,
    /// Select the handler
    Select,
}

fn main() {
    println!("Placeholder")
}
