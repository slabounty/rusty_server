use std::net::{TcpListener, TcpStream};
use std::io::{Read, Write};

use anyhow::Result;
use log::{info, error};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    info!("Rusty Server");

    // Bind the TcpListener to an address
    let listener = TcpListener::bind("127.0.0.1:8080").expect("Failed to bind to address");

    info!("Listening on 127.0.0.1:8080");

    // Accept incoming connections
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                info!("New connection: {}", stream.peer_addr().unwrap());
                handle_connection(stream)?;
            }
            Err(e) => {
                error!("Error accepting connection: {}", e);
            }
        }
    }

    Ok(())
}

fn handle_connection(mut stream: TcpStream) -> std::io::Result<()> {
    // Buffer for the request
    let mut buffer = [0; 1024];
    stream.read(&mut buffer)?;

    // Show the request in the console
    info!("Request: {}", String::from_utf8_lossy(&buffer));

    // Build a simple HTTP response
    let body = "<h1>Hello, world!</h1>";
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body
    );

    // Write the response back to the client
    stream.write_all(response.as_bytes())?;
    stream.flush()?;

    Ok(())
}
