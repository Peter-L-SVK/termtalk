use tokio::net::TcpListener;
use tokio::sync::broadcast;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::io::{BufReader, AsyncWriteExt, AsyncBufReadExt};
use termtalk::handle_client;
use std::io::Write;
use std::fs::OpenOptions;
use std::sync::Arc;
use std::collections::HashMap; // Use HashMap to store token-username mappings
use tokio::sync::Mutex; 

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Create a log file for the server
    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("server.log")?;
    let log_file = Arc::new(Mutex::new(log_file));

    // Helper function to log server messages
    async fn log_server(log_file: &Arc<Mutex<std::fs::File>>, message: &str) {
        let mut file = log_file.lock().await; // Use .await for tokio::sync::Mutex
        writeln!(&mut *file, "{}", message).expect("Failed to write to server log file");
    }

    let listener = TcpListener::bind("127.0.0.1:8080").await?;

    // Print to terminal and log to file
    println!("Server running on port 8080");
    log_server(&log_file, "Server running on port 8080").await;

    // Create a broadcast channel for message broadcasting
    let (sender, _) = broadcast::channel(32);
    
    // Atomic counter for generating unique client tokens
    let client_counter = AtomicUsize::new(0);
    
    // HashMap to store token-username mappings
    let token_username_map: Arc<Mutex<HashMap<usize, String>>> = Arc::new(Mutex::new(HashMap::new()));

    while let Ok((stream, _)) = listener.accept().await {
        // Increment the client counter to generate a unique token
        let client_token = client_counter.fetch_add(1, Ordering::SeqCst);
        log_server(&log_file, &format!("DEBUG: New client {} connected", client_token)).await;
        
        // Clone the sender and create a new receiver for each client
        let sender = sender.clone();
        let receiver = sender.subscribe();
        
        // Clone the log file for use in the spawned task
        let log_file_clone = Arc::clone(&log_file);
        
        // Clone the token-username map
        let token_username_map_clone = Arc::clone(&token_username_map);
        
        // Spawn a task to handle the client
        tokio::spawn(async move {
            let (read_stream, mut write_stream) = stream.into_split();
            let mut reader = BufReader::new(read_stream);
            let _username = String::new();
            
            // Send the client token to the client
            if write_stream
                .write_all(format!("Your token: {}\n", client_token).as_bytes())
                .await
                .is_err()
            {
                log_server(&log_file_clone, &format!("DEBUG: Failed to send token to client {}", client_token)).await;
                return; // Exit if the client disconnects
            }
            
            let mut username = String::new();
            let username_clone; // Declare username_clone outside the loop

            loop {
                username.clear();
                
                // Prompt the client for a username
                if write_stream
                    .write_all("Enter your username: ".as_bytes())
                    .await
                    .is_err()
                {
                    log_server(&log_file_clone, &format!("DEBUG: Failed to prompt client {} for username", client_token)).await;
                    return; // Exit if the client disconnects
                }
                
                // Read the username from the client
                if reader.read_line(&mut username).await.is_err() {
                    log_server(&log_file_clone, &format!("DEBUG: Failed to read username from client {}", client_token)).await;
                    return; // Exit if the client disconnects
                }
                let username = username.trim().to_string();
                
                // Check if the username is already in use by another client
                {
                    log_server(&log_file_clone, &format!("DEBUG: Checking if username '{}' is already taken", username)).await;
                    let map = token_username_map_clone.lock().await;
                    if map.values().any(|existing_username| existing_username == &username) {
                        // Send error message to the client
                        let error_message = "ERROR: Username is already taken. Please choose a different one.\n";
                        log_server(&log_file_clone, &format!("DEBUG: Username '{}' is already taken", username)).await;
                        if write_stream.write_all(error_message.as_bytes()).await.is_err() {
                            log_server(&log_file_clone, &format!("DEBUG: Failed to send error message to client {}", client_token)).await;
                        }
                        continue; // Prompt the client to enter a new username
                    }
                    log_server(&log_file_clone, &format!("DEBUG: Current usernames: {:?}", map)).await;
                } // MutexGuard is dropped here
                
                // Add the token-username mapping to the HashMap
                {
                    let mut map = token_username_map_clone.lock().await;
                    map.insert(client_token, username.clone());
                    log_server(&log_file_clone, &format!("DEBUG: Added token-username mapping: {} -> {}", client_token, username)).await;
                } // MutexGuard is dropped here
                
                // Send success message to the client
                let success_message = "SUCCESS: Username accepted.\n";
                if write_stream.write_all(success_message.as_bytes()).await.is_err() {
                    log_server(&log_file_clone, &format!("DEBUG: Failed to send success message to client {}", client_token)).await;
                    return; // Exit if the client disconnects
                }
                
                // Broadcast the welcome message only once
                let welcome_message = format!("SERVER: {} has joined the chat!\n", username);
                let _ = sender.send((username.clone(), welcome_message.clone())); // Broadcast only once
                
                // Assign the username to username_clone
                username_clone = username.clone();
                
                break; // Exit the loop once a valid username is provided
            }
            
            // Clone the username before passing it to handle_client
            let username_clone_for_log = username_clone.clone();

            // Handle the client with their chosen username and token
            handle_client(reader, write_stream, sender, receiver, username_clone, client_token).await;
            
            // Remove the token-username mapping when the client disconnects
            {
                let mut map = token_username_map_clone.lock().await;
                map.remove(&client_token);
                log_server(&log_file_clone, &format!("DEBUG: Removed token-username mapping: {} -> {}", client_token, username_clone_for_log)).await;
            } // MutexGuard is dropped here
            
            log_server(&log_file_clone, &format!("DEBUG: Client {} disconnected", client_token)).await;
        });
    }
    Ok(())
}
