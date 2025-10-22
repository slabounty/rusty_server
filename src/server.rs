// src/server.rs
use std::net::{TcpListener, TcpStream};
use anyhow::Result;
use log::{info, error};

use crate::request::{read_request, parse_request};
use crate::response::handle_response;

pub fn start_server() -> Result<()> {
    // Bind the TcpListener to an address
    let listener = TcpListener::bind("127.0.0.1:8080").expect("Failed to bind to address");
    info!("Listening on 127.0.0.1:8080");

    // Accept incoming connections
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                info!("New connection: {}", stream.peer_addr().unwrap());
                if let Err(e) = handle_connection(stream) {
                    error!("Error handling connection: {}", e);
                }
            }
            Err(e) => {
                error!("Error accepting connection: {}", e);
            }
        }
    }

    Ok(())
}

fn handle_connection(mut stream: TcpStream) -> std::io::Result<()> {
    let request_str = read_request(&mut stream)?;
    info!("request = {}", request_str);

    let request = parse_request(&request_str)?;
    info!("method = {} path = {}", request.method, request.path);

    handle_response(&mut stream, &request)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::thread;
    use std::time::Duration;
    use std::net::{TcpListener, TcpStream};


    #[test]
    fn start_server_accepts_and_responds() {
        // Start the server in a background thread
        thread::spawn(|| {
            // It runs forever, so we don’t join on it
            start_server().unwrap();
        });

        // Give the server time to start
        thread::sleep(Duration::from_millis(200));

        // Connect as a client
        let mut stream =
            TcpStream::connect("127.0.0.1:8080").expect("Failed to connect to server");

        // Send a minimal HTTP GET request
        stream
            .write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .expect("Failed to write request");

        // Read the response
        let mut response = String::new();
        stream
            .read_to_string(&mut response)
            .expect("Failed to read response");

        // Verify we got the expected body
        assert!(
            response.contains("Welcome to Rusty Server"),
            "Unexpected response: {}",
            response
        );
    }

    #[test]
    fn test_handle_connection_end_to_end() {
        // Start a listener on an ephemeral port
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        // Spawn the server in a separate thread
        thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            handle_connection(stream).unwrap();
        });

        // Simulate a client
        let mut client = TcpStream::connect(addr).unwrap();

        // Send a simple GET request
        let request = "GET / HTTP/1.1\r\nHost: localhost\r\n\r\n";
        client.write_all(request.as_bytes()).unwrap();

        // Read the server's response
        let mut response = String::new();
        client.read_to_string(&mut response).unwrap();

        // Basic validation
        assert!(response.starts_with("HTTP/1.1 200 OK"));
        assert!(response.contains("<h1>Welcome to Rusty Server</h1>"));
    }
}
