use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::time::{Duration, timeout};
use tokio::sync::broadcast;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::HashMap;
use utils::write_to_stream;

mod utils;

async fn send_user_list(write_stream: &Arc<Mutex<tokio::net::tcp::OwnedWriteHalf>>, token_username_map: &Arc<Mutex<HashMap<usize, String>>>) -> std::io::Result<()> {
    let map = token_username_map.lock().await;
    let user_list = map.values().cloned().collect::<Vec<String>>().join(", ");
    write_to_stream(write_stream, &format!("USERLIST: {}\n", user_list)).await
}

pub async fn handle_client(
    reader: BufReader<tokio::net::tcp::OwnedReadHalf>,
    write_stream: Arc<Mutex<tokio::net::tcp::OwnedWriteHalf>>,
    sender: broadcast::Sender<(String, String)>,
    mut receiver: broadcast::Receiver<(String, String)>,
    my_username: String,
    client_token: usize,
    token_username_map: Arc<Mutex<HashMap<usize, String>>>,
) {
    println!("DEBUG: Handling client {} with username: {}", client_token, my_username);

    let mut reader = reader;

    // Use the wrapped write_stream for all write operations
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
                    
                    if trimmed == "GET_USERLIST" {
                        if send_user_list(&write_stream_clone, &token_username_map).await.is_err() {
                            println!("DEBUG: Failed to send user list to client {}", client_token);
                        }
                        continue;
                    }
                    
                    // Broadcast the message only once
                    let message = format!("{}: {}\n", my_username, trimmed);
                    println!("DEBUG: Broadcasting message from client {}: {}", client_token, message);
                    let _ = sender.send((my_username.clone(), message));
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
        let disconnect_message = format!("SERVER: {} has left the chat!\n", my_username);
        println!("DEBUG: Broadcasting disconnect message: {}", disconnect_message);
        let _ = sender.send((my_username.clone(), disconnect_message));

        // Remove the user from the token_username_map
        let mut map = token_username_map.lock().await;
        map.remove(&client_token);
        println!("DEBUG: Removed token-username mapping: {} -> {}", client_token, my_username);

        // Send the updated user list to all clients
        let user_list = map.values().cloned().collect::<Vec<String>>().join(", ");
        let _ = sender.send(("SERVER".to_string(), format!("USERLIST: {}\n", user_list)));
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

    // Wait for both tasks to complete
    let (message_handler_result, broadcast_handler_result) = tokio::join!(message_handler, broadcast_handler);
    if let Err(e) = message_handler_result {
	println!("DEBUG: Message handler task failed: {:?}", e);
    }
    
    if let Err(e) = broadcast_handler_result {
	println!("DEBUG: Broadcast handler task failed: {:?}", e);
    }
    
    println!("DEBUG: Exiting handle_client for client {}", client_token);
}
