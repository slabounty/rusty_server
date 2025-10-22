use std::io::{self, Read};
use std::net::TcpStream;
use log::info;

#[derive(Debug)]
pub struct HttpRequest {
    pub method: String,
    pub path: String,
}

pub fn parse_request(request_str: &str) -> std::io::Result<HttpRequest> {
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

pub fn read_request(stream: &mut TcpStream) -> std::io::Result<String> {
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


#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::net::{TcpListener, TcpStream};
    use std::thread;

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

}
