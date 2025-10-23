use anyhow::Result;
use clap::{Parser as ClapParser};
use log::{info};

use rusty_server::cli::{Cli};
use rusty_server::start_server; // from lib.rs

fn main() -> Result<()> {
    env_logger::init();
    info!("Rusty Server");

    let cli = Cli::parse();

    let port = cli.port.unwrap_or(8080);
    info!("port = {}", port);

    let root = cli.root.as_deref().unwrap_or("static");
    info!("root = {}", root);

    start_server(port, &root)
}

#[cfg(test)]
mod tests {
}
