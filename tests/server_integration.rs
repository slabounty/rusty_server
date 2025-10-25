use std::io::{Read, Write};
use std::net::TcpStream;
use std::thread;
use std::time::Duration;
use tempfile::tempdir;
use std::fs;

// Import your server start function
use rusty_server::server::start_server;

// Helper to start the server on a separate thread
macro_rules! start_test_server {
    ($port:expr, $root:expr) => {{
        let port = $port;
        let root = $root.to_string();
        thread::spawn(move || {
            // Run the server (ignore shutdown since it runs indefinitely)
            let _ = start_server(port, &root);
        });
        // Give it a moment to start up
        std::thread::sleep(Duration::from_millis(300));
        format!("127.0.0.1:{}", port)
    }};
}

#[test]
fn server_responds_to_root_request() {
    // Create a temporary directory and index.html file
    let tmp_dir = tempdir().unwrap();
    let index_path = tmp_dir.path().join("index.html");
    fs::write(&index_path, "<h1>Hello from Test</h1>").unwrap();

    // Start the server
    let addr = start_test_server!(7878, tmp_dir.path().to_str().unwrap());

    // Connect as a client
    let mut stream = TcpStream::connect(&addr).expect("failed to connect to server");
    stream
        .write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .expect("failed to send request");

    // Read response
    let mut buffer = String::new();
    stream.read_to_string(&mut buffer).unwrap();

    assert!(
        buffer.contains("HTTP/1.1 200 OK"),
        "Expected 200 OK response, got:\n{}",
        buffer
    );
    assert!(
        buffer.contains("<h1>Hello from Test</h1>"),
        "Expected to find test HTML content, got:\n{}",
        buffer
    );
}

#[test]
fn handles_multiple_concurrent_requests() {
    // Create temp directory and a test file
    let tmp_dir = tempdir().unwrap();
    let test_file_path = tmp_dir.path().join("test.html");
    fs::write(&test_file_path, "<h1>Concurrent Test</h1>").unwrap();

    // Start server
    let addr = start_test_server!(7879, tmp_dir.path().to_str().unwrap());

    // Launch multiple threads making requests at once
    let handles: Vec<_> = (0..5)
        .map(|_| {
            let addr = addr.clone();
            thread::spawn(move || {
                let mut stream = TcpStream::connect(&addr).unwrap();
                stream
                    .write_all(b"GET /test.html HTTP/1.1\r\nHost: localhost\r\n\r\n")
                    .unwrap();

                let mut buffer = String::new();
                stream.read_to_string(&mut buffer).unwrap();
                assert!(
                    buffer.contains("HTTP/1.1 200 OK"),
                    "Expected 200 OK response, got:\n{}",
                    buffer
                );
                assert!(
                    buffer.contains("<h1>Concurrent Test</h1>"),
                    "Expected file content, got:\n{}",
                    buffer
                );
            })
        })
        .collect();

    // Wait for all threads
    for handle in handles {
        handle.join().unwrap();
    }
}

#[test]
fn server_returns_404_for_missing_file() {
    let tmp_dir = tempdir().unwrap();
    // No files are created here — we *want* a missing file

    let addr = start_test_server!(7880, tmp_dir.path().to_str().unwrap());

    let mut stream = TcpStream::connect(&addr).expect("failed to connect to server");
    stream
        .write_all(b"GET /nonexistent.html HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .expect("failed to send request");

    let mut buffer = String::new();
    stream.read_to_string(&mut buffer).unwrap();

    assert!(
        buffer.contains("HTTP/1.1 404 NOT FOUND"),
        "Expected 404 NOT FOUND, got:\n{}",
        buffer
    );
    assert!(
        buffer.contains("404"),
        "Expected body to contain 404 text, got:\n{}",
        buffer
    );
}
