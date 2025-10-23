use log::info;
use std::fs;
use std::io::{Write};
use std::net::TcpStream;
use std::path::Path;

use crate::request::HttpRequest;

pub fn handle_response(stream: &mut TcpStream, request: &HttpRequest) -> std::io::Result<()> {

    let path_str = match request.path.as_str() {
        "/" | "/index" => "static/index.html",
        other => &format!("static/{}", other.trim_start_matches('/')),
    };
    info!("Path = {}", path_str);
    let path = Path::new(path_str);


    // Detect content type
    let content_type = match path.extension().and_then(|ext| ext.to_str()) {
        Some("html") => "text/html",
        Some("css")  => "text/css",
        Some("js")   => "application/javascript",
        Some("png")  => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif")  => "image/gif",
        _ => "application/octet-stream",
    };

    // Read the file contents as bytes
    let (status_line, body) = match fs::read(&path) {
        Ok(contents) => ("HTTP/1.1 200 OK", contents),
        Err(_) => ("HTTP/1.1 404 NOT FOUND", handle_404())
    };

    // Build and send the response
    let header = format!(
        "{status_line}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\n\r\n",
        body.len()
    );

    stream.write_all(header.as_bytes())?;
    stream.write_all(&body)?;
    stream.flush()?;

    Ok(())
}

fn handle_404() -> Vec<u8> {
    let path_str = "static/404.html";
    let path = Path::new(path_str);

    // Read the 404 file and if it's not there, just generate one.
    match fs::read(&path) {
        Ok(contents) => contents,
        Err(_) => {
            b"<h1>404 Not Found</h1>".to_vec()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

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
                path: "/index.html".to_string(),
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
                path: "/about.html".to_string(),
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
    fn test_handle_response_crow_path() {
        use std::io::{Read};
        use std::net::{TcpListener, TcpStream};
        use std::thread;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let handle = thread::spawn(move || {
            let (mut server_stream, _) = listener.accept().unwrap();
            let request = HttpRequest {
                method: "GET".to_string(),
                path: "/crow.html".to_string(),
            };
            handle_response(&mut server_stream, &request).unwrap();
        });

        let mut client_stream = TcpStream::connect(addr).unwrap();
        let mut response = String::new();
        client_stream.read_to_string(&mut response).unwrap();

        handle.join().unwrap();

        assert!(response.contains("200 OK"), "Expected status line");
        assert!(response.contains("<h2>This is the crow.html file.</h2>"));
        assert!(response.contains("<img src=\"crow.jpeg\">"));
    }


    #[test]
    fn test_handle_response_jpeg() {
        use std::io::Read;
        use std::net::{TcpListener, TcpStream};
        use std::thread;

        // Bind to a random available port
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        // Spawn the server thread
        let handle = thread::spawn(move || {
            let (mut server_stream, _) = listener.accept().unwrap();
            let request = HttpRequest {
                method: "GET".to_string(),
                path: "/crow.jpeg".to_string(),
            };
            handle_response(&mut server_stream, &request).unwrap();
        });

        // Connect as the client
        let mut client_stream = TcpStream::connect(addr).unwrap();

        // Read full response into a byte buffer
        let mut buffer = Vec::new();
        client_stream.read_to_end(&mut buffer).unwrap();

        handle.join().unwrap();

        // Convert headers to text (stop at the first empty line)
        let response_text = String::from_utf8_lossy(&buffer);

        // Now you can safely assert headers or known text parts
        assert!(response_text.contains("200 OK"), "Expected HTTP 200");
        assert!(response_text.contains("Content-Type: image/jpeg"), "Expected JPEG content type");
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
            response.contains("This is the 404 file."),
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

    #[test]
    fn test_handle_404_file_exists() {
        // Create a temporary directory that mimics the project structure
        let dir = tempdir().unwrap();
        let static_dir = dir.path().join("static");
        fs::create_dir_all(&static_dir).unwrap();

        // Write a temporary 404.html file
        let expected_content = b"<h1>Custom 404 Page</h1>";
        let file_path = static_dir.join("404.html");
        fs::write(&file_path, expected_content).unwrap();

        // Temporarily change the working directory so handle_404() finds our temp file
        let old_cwd = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        // Call the function
        let result = handle_404();

        // Restore working directory
        std::env::set_current_dir(old_cwd).unwrap();

        // Assert: it should read the file content
        assert_eq!(result, expected_content, "Should return contents of 404.html");
    }

    #[test]
    fn test_handle_404_file_missing() {
        // Create a temporary directory with no 404.html
        let dir = tempdir().unwrap();
        let static_dir = dir.path().join("static");
        fs::create_dir_all(&static_dir).unwrap();

        // Temporarily change working directory so handle_404() looks here
        let old_cwd = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        // Call the function (no static/404.html exists)
        let result = handle_404();

        // Restore working directory
        std::env::set_current_dir(old_cwd).unwrap();

        // Assert: it should return the fallback HTML
        assert_eq!(
            result,
            b"<h1>404 Not Found</h1>",
            "Should return default 404 content when file missing"
        );
    }
}
