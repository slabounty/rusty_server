use std::fs;
use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;

use anyhow::Result;
use log::{info, error};

#[derive(Debug)]
struct HttpRequest {
    method: String,
    path: String,
}

fn main() -> Result<()> {
    env_logger::init();
    info!("Rusty Server");

    start_server()
}

fn start_server() -> Result<()> {
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
    // Map the request path to a file under ./static/
    info!("path = {}", request.path);

    let path = match request.path.as_str() {
        "/" => Path::new("static/index.html").to_path_buf(),
        "/index" => Path::new("static/index.html").to_path_buf(),
        "/about" => Path::new("static/about.html").to_path_buf(),
        other => {
            // Strip leading slash and append to static/
            Path::new("static").join(&other.trim_start_matches('/'))
        }
    };

    // Try to read the file contents
    let (status_line, body) = match fs::read_to_string(&path) {
        Ok(contents) => ("HTTP/1.1 200 OK", contents),
        Err(_) => ("HTTP/1.1 404 NOT FOUND", "<h1>404 Not Found</h1>".to_string()),
    };

    // Build and send the response
    let response = format!(
        "{status_line}\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );

    stream.write_all(response.as_bytes())?;
    stream.flush()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

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

    #[test]
    fn test_parse_request_valid() {
        let request_str = "GET /index.html HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let req = parse_request(request_str).unwrap();

        assert_eq!(req.method, "GET");
        assert_eq!(req.path, "/index.html");
    }

    #[test]
    fn test_parse_request_root() {
        let request_str = "GET / HTTP/1.1\r\n\r\n";
        let req = parse_request(request_str).unwrap();

        assert_eq!(req.method, "GET");
        assert_eq!(req.path, "/");
    }

    #[test]
    fn test_parse_request_malformed() {
        // Missing path
        let request_str = "GET\r\n\r\n";
        let err = parse_request(request_str).unwrap_err();

        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    }

    #[test]
    fn test_parse_request_empty() {
        // Completely empty request
        let request_str = "";
        let err = parse_request(request_str).unwrap_err();

        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    }

    #[test]
    fn test_read_request_reads_until_headers_end() {
        // Start a listener on an ephemeral port
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        // Spawn server thread to accept connection and run read_request
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let result = read_request(&mut stream).unwrap();
            result
        });

        // Connect as a client
        let mut client = TcpStream::connect(addr).unwrap();
        let request = b"GET /test HTTP/1.1\r\nHost: localhost\r\n\r\nExtra body maybe";
        client.write_all(request).unwrap();

        // Get the result from the server side
        let received = handle.join().unwrap();

        // Verify the output
        assert!(received.contains("GET /test HTTP/1.1"));
        assert!(received.contains("Host: localhost"));
        assert!(received.contains("\r\n\r\n"));
    }

    #[test]
    fn test_read_request_errors_on_incomplete_headers() {
        use std::io::Write;
        use std::net::{TcpListener, TcpStream};
        use std::thread;

        // Bind to a local ephemeral port
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        // Spawn a thread that will accept one connection and attempt to read
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            read_request(&mut stream)
        });

        // Connect as client and send an *incomplete* HTTP request (no \r\n\r\n)
        let mut client = TcpStream::connect(addr).unwrap();
        client.write_all(b"GET /incomplete HTTP/1.1\r\nHost: test").unwrap();
        // Drop client immediately to simulate abrupt close
        drop(client);

        // Server side should return an UnexpectedEof error
        let result = handle.join().unwrap();
        assert!(result.is_err(), "Expected read_request to fail on incomplete request");
        let err = result.err().unwrap();
        assert_eq!(err.kind(), std::io::ErrorKind::UnexpectedEof);
    }


    #[test]
    fn test_read_request_handles_large_headers() {
        use std::io::Write;
        use std::net::{TcpListener, TcpStream};
        use std::thread;

        // Bind a listener to an ephemeral port
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        // Spawn the server thread to accept and read the request
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            read_request(&mut stream)
        });

        // Construct a long request line + many headers
        let long_header_value = "X-Custom-Header: ".to_owned() + &"A".repeat(600);
        let full_request = format!(
            "GET /big HTTP/1.1\r\n{}\r\n\r\n",
            long_header_value
        );

        // Client: connect and send in two chunks to simulate TCP fragmentation
        let mut client = TcpStream::connect(addr).unwrap();
        let mid = full_request.len() / 2;
        client.write_all(&full_request.as_bytes()[..mid]).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(50)); // small delay
        client.write_all(&full_request.as_bytes()[mid..]).unwrap();

        // Wait for server and get result
        let result = handle.join().unwrap();
        assert!(result.is_ok(), "Expected read_request to succeed on multi-chunk request");

        let request_str = result.unwrap();
        assert!(request_str.contains("X-Custom-Header:"), "Header missing from combined read");
        assert!(request_str.ends_with("\r\n\r\n"), "Should read until end of headers");
    }


    #[test]
    fn test_handle_response_root_path() {
        use std::io::{Read};
        use std::net::{TcpListener, TcpStream};
        use std::thread;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let handle = thread::spawn(move || {
            let (mut server_stream, _) = listener.accept().unwrap();
            let request = HttpRequest {
                method: "GET".to_string(),
                path: "/".to_string(),
            };
            handle_response(&mut server_stream, &request).unwrap();
        });

        let mut client_stream = TcpStream::connect(addr).unwrap();
        let mut response = String::new();
        client_stream.read_to_string(&mut response).unwrap();

        handle.join().unwrap();

        assert!(response.contains("200 OK"), "Expected status line");
        assert!(response.contains("<h1>Welcome to Rusty Server</h1>"));
    }

    #[test]
    fn test_handle_response_index_path() {
        use std::io::{Read};
        use std::net::{TcpListener, TcpStream};
        use std::thread;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let handle = thread::spawn(move || {
            let (mut server_stream, _) = listener.accept().unwrap();
            let request = HttpRequest {
                method: "GET".to_string(),
                path: "/index".to_string(),
            };
            handle_response(&mut server_stream, &request).unwrap();
        });

        let mut client_stream = TcpStream::connect(addr).unwrap();
        let mut response = String::new();
        client_stream.read_to_string(&mut response).unwrap();

        handle.join().unwrap();

        assert!(response.contains("200 OK"), "Expected status line");
        assert!(response.contains("<h2>This is the index.html file.</h2>"));
    }

    #[test]
    fn test_handle_response_about_path() {
        use std::io::{Read};
        use std::net::{TcpListener, TcpStream};
        use std::thread;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let handle = thread::spawn(move || {
            let (mut server_stream, _) = listener.accept().unwrap();
            let request = HttpRequest {
                method: "GET".to_string(),
                path: "/about".to_string(),
            };
            handle_response(&mut server_stream, &request).unwrap();
        });

        let mut client_stream = TcpStream::connect(addr).unwrap();
        let mut response = String::new();
        client_stream.read_to_string(&mut response).unwrap();

        handle.join().unwrap();

        assert!(response.contains("200 OK"), "Expected status line");
        assert!(response.contains("<h2>This is the about.html file.</h2>"));
    }


    #[test]
    fn test_handle_response_not_found() {
        use std::io::{Read};
        use std::net::{TcpListener, TcpStream};
        use std::thread;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let handle = thread::spawn(move || {
            let (mut server_stream, _) = listener.accept().unwrap();
            let request = HttpRequest {
                method: "GET".to_string(),
                path: "/unknown".to_string(),
            };
            handle_response(&mut server_stream, &request).unwrap();
        });

        let mut client_stream = TcpStream::connect(addr).unwrap();
        let mut response = String::new();
        client_stream.read_to_string(&mut response).unwrap();

        handle.join().unwrap();

        assert!(
            response.contains("404 NOT FOUND"),
            "Expected HTTP 404 line, got: {}",
            response
        );
        assert!(
            response.contains("404 Not Found"),
            "Expected 404 body, got: {}",
            response
        );
    }


    #[test]
    fn test_handle_response_content_length_correct() {
        use std::io::{Read};
        use std::net::{TcpListener, TcpStream};
        use std::thread;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let handle = thread::spawn(move || {
            let (mut server_stream, _) = listener.accept().unwrap();
            let request = HttpRequest {
                method: "GET".to_string(),
                path: "/".to_string(),
            };
            handle_response(&mut server_stream, &request).unwrap();
        });

        let mut client_stream = TcpStream::connect(addr).unwrap();
        let mut response = String::new();
        client_stream.read_to_string(&mut response).unwrap();
        handle.join().unwrap();

        let len_line = response.lines().find(|l| l.starts_with("Content-Length")).unwrap();
        let len_val: usize = len_line.split(':').nth(1).unwrap().trim().parse().unwrap();
        let body = response.split("\r\n\r\n").nth(1).unwrap();
        assert_eq!(body.len(), len_val, "Content-Length header should match actual body size");
    }
}
