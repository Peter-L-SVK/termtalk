use tokio::net::TcpListener;
use tokio::sync::broadcast;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::io::{BufReader, AsyncBufReadExt};
use std::fs::OpenOptions;
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::Mutex;
use termtalk::handle_client;
use logging::log_message;
use utils::write_to_stream;

mod logging;
mod utils;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Create a log file for the server
    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("server.log")?;
    let log_file = Arc::new(Mutex::new(log_file));

    log_message(&log_file, "Server running on port 8080").await;

    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    let (sender, _) = broadcast::channel(32);
    let client_counter = AtomicUsize::new(0);
    let token_username_map: Arc<Mutex<HashMap<usize, String>>> = Arc::new(Mutex::new(HashMap::new()));

    while let Ok((stream, _)) = listener.accept().await {
        let client_token = client_counter.fetch_add(1, Ordering::SeqCst);
        log_message(&log_file, &format!("DEBUG: New client {} connected", client_token)).await;

        let sender = sender.clone();
        let receiver = sender.subscribe();
        let log_file_clone = Arc::clone(&log_file);
        let token_username_map_clone = Arc::clone(&token_username_map);

        tokio::spawn(async move {
            let (read_stream, write_stream) = stream.into_split();
            let mut reader = BufReader::new(read_stream);

            // Wrap write_stream in an Arc<Mutex> once at the beginning
            let write_stream = Arc::new(Mutex::new(write_stream));

            // Send the client token to the client
            if write_to_stream(&write_stream, &format!("Your token: {}\n", client_token)).await.is_err() {
                log_message(&log_file_clone, &format!("DEBUG: Failed to send token to client {}", client_token)).await;
                return;
            }

            let mut username = String::new();
            let username_clone;

            loop {
                username.clear();
                if write_to_stream(&write_stream, "Enter your username: ").await.is_err() {
                    log_message(&log_file_clone, &format!("DEBUG: Failed to prompt client {} for username", client_token)).await;
                    return;
                }
                if reader.read_line(&mut username).await.is_err() {
                    log_message(&log_file_clone, &format!("DEBUG: Failed to read username from client {}", client_token)).await;
                    return;
                }
                let username = username.trim().to_string();

                // Check if the username is already in use by another client
                {
                    let map = token_username_map_clone.lock().await;
                    if map.values().any(|existing_username| existing_username == &username) {
                        let error_message = "ERROR: Username is already taken. Please choose a different one.\n";
                        log_message(&log_file_clone, &format!("DEBUG: Username '{}' is already taken", username)).await;
                        if write_to_stream(&write_stream, &error_message).await.is_err() {
                            log_message(&log_file_clone, &format!("DEBUG: Failed to send error message to client {}", client_token)).await;
                        }
                        continue; // Prompt the client to enter a new username
                    }
                }

                // Add the token-username mapping to the HashMap
                {
                    let mut map = token_username_map_clone.lock().await;
                    map.insert(client_token, username.clone());
                    log_message(&log_file_clone, &format!("DEBUG: Added token-username mapping: {} -> {}", client_token, username)).await;
                }

                // Send success message to the client
                let success_message = "SUCCESS: Username accepted.\n";
                if write_to_stream(&write_stream, &success_message).await.is_err() {
                    log_message(&log_file_clone, &format!("DEBUG: Failed to send success message to client {}", client_token)).await;
                    return;
                }

                // Broadcast the welcome message
                let welcome_message = format!("SERVER: {} has joined the chat!\n", username);
                let _ = sender.send((username.clone(), welcome_message.clone()));
                username_clone = username.clone();
                break;
            }

            // Clone username_clone before passing it to handle_client
            let username_clone_for_log = username_clone.clone();

            // Clone token_username_map_clone before passing it to handle_client
            let token_username_map_for_handle = Arc::clone(&token_username_map_clone);

            // Handle the client with their chosen username and token
            handle_client(
                reader,
                write_stream,
                sender,
                receiver,
                username_clone,
                client_token,
                token_username_map_for_handle, // Use the cloned Arc here
            ).await;

            // Remove the token-username mapping when the client disconnects
            {
                let mut map = token_username_map_clone.lock().await;
                map.remove(&client_token);
                log_message(&log_file_clone, &format!("DEBUG: Removed token-username mapping: {} -> {}", client_token, username_clone_for_log)).await;
            }

            log_message(&log_file_clone, &format!("DEBUG: Client {} disconnected", client_token)).await;
        });
    }
    Ok(())
}
