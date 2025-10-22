use anyhow::Result;
use log::{info};

use rusty_server::start_server; // from lib.rs

fn main() -> Result<()> {
    env_logger::init();
    info!("Rusty Server");

    start_server()
}

#[cfg(test)]
mod tests {
}
