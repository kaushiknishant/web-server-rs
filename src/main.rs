use mysql::prelude::*;
use mysql::*;
use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};
const IP_ADDR: &str = "127.0.0.1:7878";
const CONN_STR: &str = "<PUT String to connect to DB";
fn main() {
    // Bind server
    match TcpListener::bind(IP_ADDR) {
        Ok(listener) => {
            println!("Server running on {}", IP_ADDR);
            for stream in listener.incoming() {
                match stream {
                    Ok(s) => handle_connection(s).expect("Found"),
                    Err(e) => println!("Connection failed: {}", e),
                }
            }
        }
        Err(e) => {
            println!("Failed to bind to address: {}", e);
        }
    }
}

fn handle_connection(mut stream: TcpStream) -> std::io::Result<()> {
    let mut buffer = [0; 1024];
    let bytes_read = match stream.read(&mut buffer) {
        Ok(0) => {
            // Client disconnected
            println!("Client closed the connection.");
            return Ok(());
        }
        Ok(n) => n,
        Err(e) => {
            eprintln!("Failed to read from stream: {}", e);
            return Err(e);
        }
    };

    let get = b"GET / HTTP/1.1\r\n";

    if buffer[..bytes_read].starts_with(get) {
        let body : String = fetch_from_db().unwrap_or_else(|e| {
            eprintln!("DB error: {}", e);
            "<h1>500 - Internal Server Error</h1>".to_string()
        });

        let response = format!("HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n{body}");

        if let Err(e) = stream.write_all(response.as_bytes()) {
            eprintln!("Failed to write response: {}", e);
        }
    } else {
        let response = "HTTP/1.1 404 NOT FOUND\r\n\r\n<h1>404 - Not Found</h1>";
        if let Err(e) = stream.write_all(response.as_bytes()) {
            eprintln!("Failed to write 404 response: {}", e);
        }
    }

    // Flush the stream
    if let Err(e) = stream.flush() {
        eprintln!("Failed to flush stream: {}", e);
    }

    Ok(())
}

fn fetch_from_db() -> Result<String, mysql::Error> {

    // Connect to MySQL
    let url = CONN_STR;
    let pool = Pool::new(url)?;
    let mut conn = pool.get_conn()?;

    // Run query (example: select all users)
    let selected: Vec<(u32, String)> = conn.query("SELECT id, name FROM users LIMIT 5")?;

    // Format into HTML
    let mut html = String::from("<!DOCTYPE html><html><body><h1>Users</h1><ul>");
    for (id, name) in selected {
        html.push_str(&format!("<li>{id}: {name}</li>"));
    }
    html.push_str("</ul></body></html>");

    Ok(html)
}
