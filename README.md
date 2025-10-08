# Rusty Server
A rust based simple http server

## ⏳ Hour-by-Hour Breakdown
### Hour 1 – Project Setup & Hello-Server

Create project:

cargo new static_server
cd static_server


In main.rs:

Import std::net::TcpListener and std::io::{Read, Write}.

Bind to 127.0.0.1:8080.

Accept a single connection, read a few bytes, and print them.

Respond with a hard-coded HTTP response:

let response = "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\nHello!";
stream.write_all(response.as_bytes())?;


Verify in a browser: http://127.0.0.1:8080.

### Hour 2 – Continuous Loop & Basic Request Parsing

Wrap the accept in a for stream in listener.incoming() loop.

For each connection:

Read until \r\n\r\n.

Extract the request line (GET /path HTTP/1.1).

Parse method and path into variables.

Print each request to stdout.

### Hour 3 – Static Directory & Path Resolution

Create a static/ directory:

static/
  ├─ index.html
  └─ about.html


In code:

Map / → static/index.html.

Map /about → static/about.html.

Default to 404 if file not found.

### Hour 4 – Serving Files

Use std::fs::read to load file contents as Vec<u8>.

Respond with:

HTTP/1.1 200 OK
Content-Type: text/html
Content-Length: <len>

<file-contents>


Detect Content-Type by extension (.html, .css, .png) with a simple match statement.

Test by opening pages in a browser.

### Hour 5 – Concurrency with Threads

Wrap connection handling in:

std::thread::spawn(|| handle_client(stream));


Implement a handle_client function that:

Reads the request,

Figures out the file path,

Writes the response.

Add basic error handling (log errors instead of crashing).

### Hour 6 – Logging & Config

Add a log_request helper: log IP, method, path, status.

Add CLI argument for port number using just std::env::args().

Print “Server running on 127.0.0.1:PORT”.

### Hour 7 – Error Responses & Refactor

Create reusable HttpResponse struct:

struct HttpResponse {
    status_line: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}


Add to_bytes() to build a raw HTTP response.

Create a custom 404.html page and serve it for missing files.

Move code into:

src/main.rs → startup & listener

src/server.rs → connection loop

src/response.rs → HttpResponse

src/utils.rs → MIME detection, path helpers.

### Hour 8 – Unit Tests

Write unit tests for:

get_mime_type("file.html")

HttpResponse::to_bytes()

Path-resolution helper.

Use #[cfg(test)] blocks inside each module.

### Hour 9 – Integration Tests

Create tests/ folder:

tests/server_integration.rs


Start the server in a thread with a known port.

Use std::net::TcpStream or reqwest (add as dev-dependency) to send requests.

Assert that status code and body are correct for / and /404.

### Hour 10 – Polish & README

Add graceful shutdown (detect Ctrl+C → close listener).

Improve logging with timestamps.

Write a short README.md describing:

How to build/run the server.

How to add new static pages.

How to run tests.

(Optional) Add a Dockerfile.
