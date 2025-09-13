use mysql::prelude::*;
use mysql::*;
use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;

const IP_ADDR: &str = "127.0.0.1:7878";
const CONN_STR: &str = "mysql://maintainer:maintainer@12@localhost:3306/userbase";
fn main() {

    // Create the pool once and wrap in Arc so threads can share it
    let pool = match Pool::new(CONN_STR) {
        Ok(p) => Arc::new(p),
        Err(e) => {
            eprintln!("Failed to create DB pool: {}", e);
            return;
        }
    };

    // Bind server
    match TcpListener::bind(IP_ADDR) {
        Ok(listener) => {
            println!("Server running on {}", IP_ADDR);
            for stream in listener.incoming() {
                match stream {
                    Ok(s) => {
                        // clone Arc so each connection handler gets a ref to the pool
                        let pool = Arc::clone(&pool);
                        if let Err(e) = handle_connection(s, pool) {
                            eprintln!("Connection handler error: {}", e);
                        }
                    }
                    Err(e) => println!("Connection failed: {}", e),
                }
            }
        }
        Err(e) => {
            println!("Failed to bind to address: {}", e);
        }
    }
}

fn handle_connection(mut stream: TcpStream, pool: Arc<Pool>) -> std::io::Result<()> {
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
        let body : String = fetch_from_db(&pool).unwrap_or_else(|e| {
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

fn fetch_from_db(pool: &Pool) -> Result<String, mysql::Error> {

    // Connect to MySQL
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


#[cfg(test)]
mod tests {
    use super::*;
    use mysql::*;

    #[test]
    fn test_fetch_from_db_returns_html() {
       // create pool (assumes testdb exists and has `users` table)
        let pool = Pool::new(CONN_STR).expect("Failed to connect to DB");

        let result = fetch_from_db(&pool);

        match result {
            Ok(html) => {
                assert!(html.contains("<html>"));
                assert!(html.contains("<ul>"));
                // If you have test data, you can assert known values
                // e.g., assert!(html.contains("Alice"));
            }
            Err(e) => panic!("DB query failed: {}", e),
        }
    }

    use super::*;
    use std::thread;
    use std::io::{Read, Write};


    #[test]
    fn test_handle_connection_root_request() {
        // create a pool
        let pool = Arc::new(Pool::new(CONN_STR).expect("DB pool failed"));

        // Start a background thread to simulate client
        let listener = TcpListener::bind("127.0.0.1:0").unwrap(); // bind random port
        let addr = listener.local_addr().unwrap();
        let pool_clone = Arc::clone(&pool);

        thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                handle_connection(stream, pool_clone).unwrap();
            }
        });

        // Act: client side
        let mut client = TcpStream::connect(addr).unwrap();
        client.write_all(b"GET / HTTP/1.1\r\n\r\n").unwrap();

        // Read response
        let mut buffer = String::new();
        client.read_to_string(&mut buffer).unwrap();

        // Assert
        assert!(buffer.contains("HTTP/1.1 200 OK"));
    }
}
