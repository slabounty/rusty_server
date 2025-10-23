use clap::{Parser as ClapParser};

#[derive(ClapParser, Default)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Sets the root root (default 8080)
    #[arg(short, long, value_name = "DIRECTORY")]
    pub root: Option<String>,

    /// Sets the port for the server to use (default static/)
    #[arg(short, long, value_name = "PORT")]
    pub port: Option<u16>,
}
