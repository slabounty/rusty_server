use log::info;
use std::fs;
use std::io::{Write};
use std::path::{Path, PathBuf};

use crate::request::HttpRequest;

//pub fn handle_response(stream: &mut TcpStream, request: &HttpRequest, root: &str) -> std::io::Result<()> {
pub fn handle_response<T: Write>(mut stream: T, request: &HttpRequest, root: &str) -> std::io::Result<()> {

    info!("root = {}", root);
    let path = generate_path(request, root);
    info!("path = {}", path.display());

    let content_type = detect_mime_type(&path);

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



fn generate_path(request: &HttpRequest, root: &str) -> PathBuf {
    let mut path = PathBuf::from(root);
    let relative = match request.path.as_str() {
        "/" | "/index" => "index.html",
        other => other.trim_start_matches('/'),
    };
    path.push(relative);
    info!("Path = {:?}", path);
    path
}

fn detect_mime_type(path: &Path) -> &'static str {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("html") => "text/html",
        Some("css")  => "text/css",
        Some("js")   => "application/javascript",
        Some("png")  => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif")  => "image/gif",
        _ => "application/octet-stream",
    }
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
    use std::path::PathBuf;

    /// Helper to run `handle_response` and return the full HTTP response as a String.
    fn run_handle_response(method: &str, path: &str, static_dir: &std::path::Path) -> String {
        let mut buffer = Vec::new();
        let request = HttpRequest {
            method: method.to_string(),
            path: path.to_string(),
        };
        handle_response(&mut buffer, &request, static_dir.to_str().unwrap()).unwrap();
        String::from_utf8(buffer).unwrap()
    }

    /// Helper to create a basic static directory for testing.
    fn setup_static_dir() -> tempfile::TempDir {
        let dir = tempdir().unwrap();
        let static_dir = dir.path().join("static");
        fs::create_dir_all(&static_dir).unwrap();

        // Create simple test files
        fs::write(static_dir.join("index.html"), "<h2>This is the index.html file.</h2>").unwrap();
        fs::write(static_dir.join("about.html"), "<h2>This is the about.html file.</h2>").unwrap();
        fs::write(
            static_dir.join("crow.html"),
            "<h2>This is the crow.html file.</h2><img src=\"crow.jpeg\">",
        )
        .unwrap();
        fs::write(static_dir.join("crow.jpeg"), b"fakejpegdata").unwrap();
        fs::write(static_dir.join("404.html"), "This is the 404 file.").unwrap();
        fs::write(static_dir.join("index.txt"), "plain text file").unwrap();

        dir
    }

    #[test]
    fn test_handle_response_root_path() {
        let dir = setup_static_dir();
        let static_dir = dir.path().join("static");

        let response = run_handle_response("GET", "/", &static_dir);
        assert!(response.contains("200 OK"), "Expected status line");
        assert!(response.contains("index.html"), "Should serve index page");
    }

    #[test]
    fn test_handle_response_index_path() {
        let dir = setup_static_dir();
        let static_dir = dir.path().join("static");

        let response = run_handle_response("GET", "/index.html", &static_dir);
        assert!(response.contains("200 OK"), "Expected status line");
        assert!(response.contains("<h2>This is the index.html file.</h2>"));
    }

    #[test]
    fn test_handle_response_about_path() {
        let dir = setup_static_dir();
        let static_dir = dir.path().join("static");

        let response = run_handle_response("GET", "/about.html", &static_dir);
        assert!(response.contains("200 OK"), "Expected status line");
        assert!(response.contains("<h2>This is the about.html file.</h2>"));
    }

    #[test]
    fn test_handle_response_crow_path() {
        let dir = setup_static_dir();
        let static_dir = dir.path().join("static");

        let response = run_handle_response("GET", "/crow.html", &static_dir);
        assert!(response.contains("200 OK"), "Expected status line");
        assert!(response.contains("<h2>This is the crow.html file.</h2>"));
        assert!(response.contains("<img src=\"crow.jpeg\">"));
    }

    #[test]
    fn test_handle_response_jpeg() {
        let dir = setup_static_dir();
        let static_dir = dir.path().join("static");

        let mut buffer = Vec::new();
        let request = HttpRequest {
            method: "GET".to_string(),
            path: "/crow.jpeg".to_string(),
        };
        handle_response(&mut buffer, &request, static_dir.to_str().unwrap()).unwrap();

        let response_text = String::from_utf8_lossy(&buffer);
        assert!(response_text.contains("200 OK"), "Expected HTTP 200");
        assert!(
            response_text.contains("Content-Type: image/jpeg"),
            "Expected JPEG content type"
        );
    }

    #[test]
    fn test_handle_response_not_found() {
        let dir = setup_static_dir();
        let static_dir = dir.path().join("static");

        let response = run_handle_response("GET", "/unknown", &static_dir);
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
        let dir = setup_static_dir();
        let static_dir = dir.path().join("static");

        let response = run_handle_response("GET", "/index.html", &static_dir);
        let len_line = response
            .lines()
            .find(|l| l.starts_with("Content-Length"))
            .unwrap();
        let len_val: usize = len_line.split(':').nth(1).unwrap().trim().parse().unwrap();
        let body = response.split("\r\n\r\n").nth(1).unwrap();
        assert_eq!(
            body.len(),
            len_val,
            "Content-Length header should match actual body size"
        );
    }

    #[test]
    fn test_handle_404_file_exists() {
        let dir = tempdir().unwrap();
        let static_dir = dir.path().join("static");
        fs::create_dir_all(&static_dir).unwrap();

        let expected_content = b"<h1>Custom 404 Page</h1>";
        let file_path = static_dir.join("404.html");
        fs::write(&file_path, expected_content).unwrap();

        let old_cwd = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();
        let result = handle_404();
        std::env::set_current_dir(old_cwd).unwrap();

        assert_eq!(result, expected_content, "Should return contents of 404.html");
    }

    #[test]
    fn test_handle_404_file_missing() {
        let dir = tempdir().unwrap();
        let static_dir = dir.path().join("static");
        fs::create_dir_all(&static_dir).unwrap();

        let old_cwd = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();
        let result = handle_404();
        std::env::set_current_dir(old_cwd).unwrap();

        assert_eq!(
            result,
            b"<h1>404 Not Found</h1>",
            "Should return default 404 content when file missing"
        );
    }

    #[test]
    fn generates_index_for_root() {
        let request = HttpRequest {
            method: "GET".to_string(),
            path: "/".to_string(),
        };
        let root = "/tmp/site";

        let result = generate_path(&request, root);

        assert_eq!(result, PathBuf::from("/tmp/site/index.html"));
    }

    #[test]
    fn generates_index_for_index_path() {
        let request = HttpRequest {
            method: "GET".to_string(),
            path: "/index".to_string(),
        };
        let root = "/tmp/site";

        let result = generate_path(&request, root);

        assert_eq!(result, PathBuf::from("/tmp/site/index.html"));
    }

    #[test]
    fn generates_path_for_static_asset() {
        let request = HttpRequest {
            method: "GET".to_string(),
            path: "/css/style.css".to_string(),
        };
        let root = "/tmp/site";

        let result = generate_path(&request, root);

        assert_eq!(result, PathBuf::from("/tmp/site/css/style.css"));
    }

    #[test]
    fn trims_leading_slashes() {
        let request = HttpRequest {
            method: "GET".to_string(),
            path: "///images/logo.png".to_string(),
        };
        let root = "/tmp/site";

        let result = generate_path(&request, root);

        assert_eq!(result, PathBuf::from("/tmp/site/images/logo.png"));
    }

    #[test]
    fn test_mime_type_html() {
        let path = Path::new("somedir/somefile.html");

        let content_type = detect_mime_type(&path);

        assert_eq!(content_type, "text/html", "Expected html mimetype");
    }

    #[test]
    fn test_mime_type_css() {
        let path = Path::new("somedir/somefile.css");

        let content_type = detect_mime_type(&path);

        assert_eq!(content_type, "text/css", "Expected css mimetype");
    }

    #[test]
    fn test_mime_type_js() {
        let path = Path::new("somedir/somefile.js");

        let content_type = detect_mime_type(&path);

        assert_eq!(content_type, "application/javascript", "Expected js mimetype");
    }

    #[test]
    fn test_mime_type_png() {
        let path = Path::new("somedir/somefile.png");

        let content_type = detect_mime_type(&path);

        assert_eq!(content_type, "image/png", "Expected png mimetype");
    }

    #[test]
    fn test_mime_type_jpg() {
        let path = Path::new("somedir/somefile.jpg");

        let content_type = detect_mime_type(&path);

        assert_eq!(content_type, "image/jpeg", "Expected jpg mimetype");
    }

    #[test]
    fn test_mime_type_jpeg() {
        let path = Path::new("somedir/somefile.jpeg");

        let content_type = detect_mime_type(&path);

        assert_eq!(content_type, "image/jpeg", "Expected jpeg mimetype");
    }

    #[test]
    fn test_mime_type_gif() {
        let path = Path::new("somedir/somefile.gif");

        let content_type = detect_mime_type(&path);

        assert_eq!(content_type, "image/gif", "Expected gif mimetype");
    }

    #[test]
    fn test_mime_type_other() {
        let path = Path::new("somedir/somefile.other");

        let content_type = detect_mime_type(&path);

        assert_eq!(content_type, "application/octet-stream", "Expected other mimetype");
    }
}
