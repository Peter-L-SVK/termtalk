use tokio::sync::Mutex;
use tokio::io::AsyncWriteExt;
use std::sync::Arc;
use colored::*;
use chrono::Local;
use std::marker::Unpin; 

// Helper function to write to a stream
pub async fn write_to_stream(write_stream: &Arc<Mutex<impl AsyncWriteExt + Unpin>>, message: &str) -> std::io::Result<()> {
    let mut stream = write_stream.lock().await;
    stream.write_all(message.as_bytes()).await
}

#[allow(dead_code)] //it is used in client, just compiler is confused
pub fn format_message(_username: &str, message: &str, is_server_message: bool, my_username: &str) -> String {
    let timestamp = Local::now().format("[%d.%m.%Y %H:%M]").to_string();
    if is_server_message {
        format!("{} {}", timestamp.black(), message.magenta())
    } else {
        // Split the message into username and content
        let mut parts = message.splitn(2, ':');
        let message_username = parts.next().unwrap_or("").trim(); // Extract the username part
        let message_content = parts.next().unwrap_or("").trim(); // Extract the message content
        let colored_username = if message_username == my_username {
            message_username.green().to_string() 
        } else {
            message_username.blue().to_string() 
        };

        // Apply mention highlighting
        let formatted_content = if message_content.contains(&format!("@{}", my_username)) || message_content.contains("@all") {
            message_content
                .split_whitespace()
                .map(|word| {
                    if word == &format!("@{}", my_username) || word == "@all" {
                        word.red().bold().to_string()
                    } else {
                        word.to_string()
                    }
                })
                .collect::<Vec<String>>()
                .join(" ")
        } else {
            // Otherwise, leave the message content as is
            message_content.to_string()
        };

        // Combine the timestamp, colored username, and formatted content
        format!("{} {}: {}", timestamp.black(), colored_username, formatted_content)
    }
}
