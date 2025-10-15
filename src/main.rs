use std::net::{TcpListener, TcpStream};
use std::io::{self, Read, Write};

use anyhow::Result;
use log::{info, error};

struct HttpRequest {
    method: String,
    path: String,
}

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
    let request_str = read_request(&mut stream)?;
    info!("request = {}", request_str);

    let request = parse_request(&request_str)?;
    info!("method = {} path = {}", request.method, request.path);

    handle_response(&mut stream, &request)?;

    Ok(())
}

fn parse_request(request_str: &str) -> std::io::Result<HttpRequest> {
    if let Some(line) = request_str.lines().next() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let method = parts[0].to_string();
            let path = parts[1].to_string();
            return Ok(HttpRequest { method, path });
        }
    }

    Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Malformed request line"))
}

fn read_request(stream: &mut TcpStream) -> std::io::Result<String> {
    let mut buffer = Vec::new();
    let mut temp = [0; 512];

    // Read until we find "\r\n\r\n" (end of headers)
    loop {
        let n = stream.read(&mut temp)?;
        if n == 0 {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "connection closed"));
        }

        buffer.extend_from_slice(&temp[..n]);

        if buffer.windows(4).any(|w| w == b"\r\n\r\n") {
            break;
        }
    }

    let request_str = String::from_utf8_lossy(&buffer).to_string();
    info!("request = {}", request_str);

    Ok(request_str)
}


fn handle_response(stream: &mut TcpStream, request: &HttpRequest) -> std::io::Result<()> {
    // Example: respond differently based on path
    let body = if request.path == "/" {
        "<h1>Welcome to Rusty Server</h1>"
    } else {
        "<h1>404 Not Found</h1>"
    };

    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body
    );

    stream.write_all(response.as_bytes())?;
    stream.flush()?;
    Ok(())
}
