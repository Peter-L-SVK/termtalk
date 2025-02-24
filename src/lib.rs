use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::time::{Duration, timeout};
use tokio::sync::broadcast;
use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn handle_client(
    reader: BufReader<tokio::net::tcp::OwnedReadHalf>,
    write_stream: tokio::net::tcp::OwnedWriteHalf,
    sender: broadcast::Sender<(String, String)>,
    mut receiver: broadcast::Receiver<(String, String)>,
    my_username: String,
    client_token: usize,
) {
    println!("DEBUG: Handling client {} with username: {}", client_token, my_username);

    let mut reader = reader;

    // Wrap write_stream in an Arc<Mutex> to share it between tasks
    let write_stream = Arc::new(Mutex::new(write_stream));
    let sender_clone = sender.clone();
    let my_username_clone = my_username.clone();
    let write_stream_clone = Arc::clone(&write_stream);

    // Spawn a task to handle incoming messages from the client
    let message_handler = tokio::spawn(async move {
        println!("DEBUG: Spawning task for client {} messages and pings", client_token);
        
        let mut buf = String::new();
        loop {
            buf.clear();
            
            match timeout(Duration::from_secs(15), reader.read_line(&mut buf)).await {
                Ok(Ok(n)) if n > 0 => {
                    let trimmed = buf.trim();
                    
                    if trimmed == "PONG" {
                        println!("DEBUG: Received PONG from client {}", client_token);
                        continue;
                    }
                    
                    // Broadcast the message only once
                    let message = format!("{}: {}\n", my_username_clone, trimmed);
                    println!("DEBUG: Broadcasting message from client {}: {}", client_token, message);
                    let _ = sender_clone.send((my_username_clone.clone(), message));
                }
                Ok(Ok(_)) | Ok(Err(_)) => {
                    println!("DEBUG: Client {} disconnected", client_token);
                    break;
                }
                Err(_) => {
                    println!("DEBUG: Timeout from client {}, sending PING", client_token);
                    if write_stream_clone.lock().await.write_all(b"PING\n").await.is_err() {
                        println!("DEBUG: Failed to send PING to client {}", client_token);
                        break;
                    }
                }
            }
        }
        
        // Broadcast disconnect message only once
        let disconnect_message = format!("SERVER: {} has left the chat!\n", my_username_clone);
        println!("DEBUG: Broadcasting disconnect message: {}", disconnect_message);
        let _ = sender_clone.send((my_username_clone.clone(), disconnect_message));
    });
    
    // Spawn a task to handle broadcast messages and forward them to the client
    let write_stream_clone = Arc::clone(&write_stream);
    let broadcast_handler = tokio::spawn(async move {
        loop {
            match receiver.recv().await {
                Ok((_sender_username, message)) => {
                    // Forward ALL messages to the client, regardless of sender
                    if write_stream_clone.lock().await.write_all(message.as_bytes()).await.is_err() {
                        println!("DEBUG: Failed to forward message to client {}", client_token);
                        break;
                    }
                }
                Err(_) => {
                    println!("DEBUG: Broadcast channel closed for client {}", client_token);
                    break;
                }
            }
        }
    });

    // Wait for either task to complete
    tokio::select! {
        _ = message_handler => (),
        _ = broadcast_handler => (),
    }

    println!("DEBUG: Exiting handle_client for client {}", client_token);
}
