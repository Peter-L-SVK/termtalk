use tokio::net::TcpListener;
use tokio::sync::broadcast;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::io::{BufReader, AsyncWriteExt, AsyncBufReadExt};
use termtalk::handle_client;
use std::io::Write;
use std::fs::OpenOptions;
use std::sync::{Arc, Mutex};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Create a log file for the server
    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("server.log")?;
    let log_file = Arc::new(Mutex::new(log_file));

    // Helper function to log server messages
    fn log_server(log_file: &Arc<Mutex<std::fs::File>>, message: &str) {
        let mut file = log_file.lock().unwrap();
        writeln!(file, "{}", message).expect("Failed to write to server log file");
    }

    let listener = TcpListener::bind("127.0.0.1:8080").await?;

    // Print to terminal and log to file
    println!("Server running on port 8080");
    log_server(&log_file, "Server running on port 8080");

    // Create a broadcast channel for message broadcasting
    let (sender, _) = broadcast::channel(32);
    
    // Atomic counter for generating unique client tokens
    let client_counter = AtomicUsize::new(0);
    
    while let Ok((stream, _)) = listener.accept().await {
        // Increment the client counter to generate a unique token
        let client_token = client_counter.fetch_add(1, Ordering::SeqCst);
        log_server(&log_file, &format!("DEBUG: New client {} connected", client_token));
        
        // Clone the sender and create a new receiver for each client
        let sender = sender.clone();
        let receiver = sender.subscribe();
        
        // Clone the log file for use in the spawned task
        let log_file_clone = Arc::clone(&log_file);
        
        // Spawn a task to handle the client
        tokio::spawn(async move {
            let (read_stream, mut write_stream) = stream.into_split();
            let mut reader = BufReader::new(read_stream);
            let mut username = String::new();
            
            // Send the client token to the client
            if write_stream
                .write_all(format!("Your token: {}\n", client_token).as_bytes())
                .await
                .is_err()
            {
                log_server(&log_file_clone, &format!("DEBUG: Failed to send token to client {}", client_token));
                return; // Exit if the client disconnects
            }
            
            // Prompt the client for a username
            if write_stream
                .write_all("Enter your username: ".as_bytes())
                .await
                .is_err()
            {
                log_server(&log_file_clone, &format!("DEBUG: Failed to prompt client {} for username", client_token));
                return; // Exit if the client disconnects
            }
            
            // Read the username from the client
            if reader.read_line(&mut username).await.is_err() {
                log_server(&log_file_clone, &format!("DEBUG: Failed to read username from client {}", client_token));
                return; // Exit if the client disconnects
            }
            let username = username.trim().to_string();
            log_server(&log_file_clone, &format!("DEBUG: Client {} registered with username: {}", client_token, username));
            
            // Send welcome message to the client
            let welcome_message = format!("SERVER: {} has joined the chat!\n", username);
            let _ = sender.send((username.clone(), welcome_message.clone()));
            if write_stream.write_all(welcome_message.as_bytes()).await.is_err() {
                log_server(&log_file_clone, &format!("DEBUG: Failed to send welcome message to client {}", client_token));
                return; // Exit if the client disconnects
            }
            
            // Handle the client with their chosen username and token
            handle_client(reader, write_stream, sender, receiver, username, client_token).await;
            
            log_server(&log_file_clone, &format!("DEBUG: Client {} disconnected", client_token));
        });
    }
    Ok(())
}
