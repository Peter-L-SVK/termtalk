use chrono::Local;
use tokio::net::TcpStream;
use tokio::io::{AsyncBufReadExt};
use tokio::sync::broadcast;
use tokio::sync::Mutex;
use colored::*;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io::{self};
use std::fs::OpenOptions;
use std::sync::Arc;
use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders, Paragraph},
    text::{Text, Span, Spans},
    style::{Style, Color},
    Terminal,
};
use tokio::time::{self, Duration};

mod logging;
mod utils;

use logging::log_message;
use utils::{write_to_stream, format_message};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("client.log")?;
    let log_file = Arc::new(Mutex::new(log_file));

    // Log terminal initialization
    log_message(&log_file, "[DEBUG] Terminal initialized successfully").await;

    // Create a broadcast channel for message broadcasting
    let (sender, mut receiver) = broadcast::channel::<(String, String)>(32);

    // Connect to the server
    log_message(&log_file, "[DEBUG] Connecting to server...").await;
    let stream = match TcpStream::connect("127.0.0.1:8080").await {
        Ok(stream) => {
            log_message(&log_file, "[DEBUG] Connected to server").await;
            stream
        }
        Err(_) => {
            log_message(&log_file, "[DEBUG] Failed to connect to the server").await;
            return Ok(());
        }
    };
    let (read_stream, write_stream) = stream.into_split();
    let mut reader = tokio::io::BufReader::new(read_stream);

    // Wrap write_stream in an Arc<Mutex> for shared ownership
    let write_stream = Arc::new(Mutex::new(write_stream));

    // Read the client token from the server
    let mut token_message = String::new();
    if reader.read_line(&mut token_message).await.is_err() {
        log_message(&log_file, "[DEBUG] Failed to read token from server").await;
        return Ok(());
    }
    let client_token = token_message.trim().split_whitespace().last().unwrap_or("unknown").to_string();
    log_message(&log_file, &format!("[DEBUG] Client token: {}", client_token)).await;

    // Prompt the client for a username
    let mut username = String::new();
    let mut error_message = String::new();
    loop {
	terminal.draw(|f| {
            let size = f.size();
            let chunks = Layout::default()
		.direction(Direction::Vertical)
		.constraints(
                    [
			Constraint::Percentage(80),
			Constraint::Percentage(20),
                    ]
			.as_ref(),
		)
		.split(size);
            
            // Render username input area
            let input_block = Paragraph::new(format!("Enter your username: {}\n\nPress 'Esc' to quit.", username))
		.block(Block::default().borders(Borders::ALL));
            f.render_widget(input_block, chunks[0]);
            
            // Render error message area
            let error_block = Paragraph::new(Text::from(Spans::from(vec![
		Span::styled(
                    error_message.clone(),
                    Style::default().fg(Color::Red),
		)
            ])))
		.block(Block::default().borders(Borders::ALL));
            f.render_widget(error_block, chunks[1]);
	})?;
	
	if let Event::Key(key) = event::read()? {
            match key.code {
		KeyCode::Enter => {
                    if username.trim().is_empty() {
			error_message = "Error: Username cannot be empty!".to_string();
                    } else {
			if write_to_stream(&write_stream, &format!("{}\n", username)).await.is_err() {
                            log_message(&log_file, "[DEBUG] Failed to send username to server").await;
                            return Ok(());
			}
			
			let mut response = String::new();
			if reader.read_line(&mut response).await.is_err() {
                            log_message(&log_file, "[DEBUG] Failed to read server response").await;
                            return Ok(());
			}
			
			// Log the server's response
			log_message(&log_file, &format!("[DEBUG] Server response: {}", response.trim())).await;
			
			// Filter out the "Enter your username: " prefix from the response
			let response = response.trim().strip_prefix("Enter your username: ").unwrap_or(response.trim());
			
			// Check if the server accepted the username
			if response == "SUCCESS: Username accepted." {
                            break; 
			} else if response == "ERROR: Username is already taken. Please choose a different one." {
                            error_message = response.to_string();
                            username.clear(); 
                            continue; 
			} else {
                            error_message = "Unexpected server response. Please try again.".to_string();
                            username.clear(); 
                            continue; 
			}
                    }
		}
		KeyCode::Backspace => {
                    username.pop();
                    error_message.clear();
		}
		KeyCode::Char(c) => {
                    username.push(c);
                    error_message.clear();
		}
		KeyCode::Esc => {
                    // Quit the application immediately
                    log_message(&log_file, "[DEBUG] Quitting application from login screen").await;
                    disable_raw_mode()?;
                    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
                    return Ok(());
		}
		_ => {}
            }
	}
    }
    
    // Transition to chat state
    log_message(&log_file, "[DEBUG] Transitioning to chat state").await;
    terminal.clear()?;
    
    // Spawn a task to handle the client
    let sender_clone = sender.clone();
    let username_clone = username.clone();
    let log_file_clone = Arc::clone(&log_file);
    let client_token_clone = client_token.clone();
    let write_stream_clone = Arc::clone(&write_stream);
    tokio::spawn(async move {
        log_message(&log_file_clone, &format!("[DEBUG] Spawning client handler task for token {}", client_token_clone)).await;
        let mut input = String::new();
        loop {
            input.clear();
            if reader.read_line(&mut input).await.is_err() {
                log_message(&log_file_clone, "[DEBUG] Disconnected from server").await;
                break;
            }
	    
            if input.trim() == "PING" {
                if write_to_stream(&write_stream_clone, "PONG\n").await.is_err() {
                    log_message(&log_file_clone, "[DEBUG] Failed to send pong to server").await;
                    break;
                }
                continue;
            }
	    
            let timestamp = Local::now().format("[%d.%m.%Y %H:%M]").to_string();
            let message = format!("{} {}", timestamp.black(), input.trim());
            log_message(&log_file_clone, &format!("[DEBUG] Broadcasting message from token {}: {}", client_token_clone, message)).await;
            let _ = sender_clone.send((username_clone.clone(), input.trim().to_string()));
        }
        log_message(&log_file_clone, &format!("[DEBUG] Client handler task for token {} exited", client_token_clone)).await;
    });
    
    // Chat UI loop
    let mut messages: Vec<String> = Vec::new();
    let mut input_text = String::new();
    let mut scroll_offset: u16 = 0;
    let mut interval = time::interval(Duration::from_millis(100));
    let mut show_user_list = false;
    loop {
	tokio::select! {
            _ = interval.tick() => {
		terminal.draw(|f| {
                    let chunks = Layout::default()
			.direction(Direction::Vertical)
			.constraints(
                            [
				Constraint::Percentage(90),
				Constraint::Percentage(10),
                            ]
				.as_ref(),
			)
			.split(f.size());
		    
                    if show_user_list {
			// Render the user list screen
			let user_list: Vec<&String> = messages.iter().filter(|msg| msg.starts_with("USERLIST:")).collect();
			let user_list_text = user_list.iter().map(|s| s.as_str()).collect::<Vec<&str>>().join("\n");
			let user_list_block = Paragraph::new(format!("{}\n\nPress 'r' to return to chat.", user_list_text))
                            .block(Block::default().borders(Borders::ALL).title("User List"))
                            .scroll((scroll_offset, 0));
			f.render_widget(user_list_block, chunks[0]);
                    } else {
			// Render the chat screen, filtering out USERLIST: messages
			let chat_messages: Vec<&String> = messages.iter().filter(|msg| !msg.starts_with("USERLIST:")).collect();
			let chat_messages_text = chat_messages.iter().map(|s| s.as_str()).collect::<Vec<&str>>().join("\n");
			let message_block = Paragraph::new(chat_messages_text)
                            .block(Block::default().borders(Borders::ALL))
                            .scroll((scroll_offset, 0));
			f.render_widget(message_block, chunks[0]);
                    }
		    
                    // Render the input block with the username prefix
                    let input_block = Paragraph::new(format!("{}: {}", username, input_text))
			.block(Block::default().borders(Borders::ALL));
                    f.render_widget(input_block, chunks[1]);
		})?;
            }
            event = async { event::read() } => {
		if let Event::Key(key) = event? {
                    match key.code {
			KeyCode::Enter => {
                            if !input_text.is_empty() {
				if write_to_stream(&write_stream, &format!("{}\n", input_text.trim())).await.is_err() {
                                    log_message(&log_file, "[DEBUG] Failed to send message to server").await;
                                    break;
				}
				input_text.clear();
                            }
			}
			KeyCode::Backspace => {
                            input_text.pop();
			}
			KeyCode::Char(c) => {
                            if key.modifiers.contains(event::KeyModifiers::CONTROL) {
				match c {
                                    'l' => {
					// Request the user list from the server
					if write_to_stream(&write_stream, "GET_USERLIST\n").await.is_err() {
                                            log_message(&log_file, "[DEBUG] Failed to request user list from server").await;
                                            break;
					}
					show_user_list = true;
                                    }
                                    _ => {} // Ignore other Ctrl combinations
				}
                            } else if show_user_list && c == 'r' {
				show_user_list = false;
                            } else {
				input_text.push(c);
                            }
			}
			KeyCode::Up => {
                            scroll_offset = scroll_offset.saturating_sub(1);
			}
			KeyCode::Down => {
                            scroll_offset = scroll_offset.saturating_add(1);
			}
			KeyCode::Esc => break,
			_ => {}
                    }
		}
            }
	}
	
	while let Ok((sender_username, message)) = receiver.try_recv() {
            if message.starts_with("USERLIST:") {
		// Update the user list
		let user_list = message.trim_start_matches("USERLIST: ").to_string();
		messages.retain(|msg| !msg.starts_with("USERLIST:")); 
		messages.push(format!("USERLIST: {}", user_list)); 
		continue;
            }
	    
            if message.contains("Enter your username:") {
		continue;
            }
	    
            let is_server_message = message.starts_with("SERVER:");
            let formatted_message = format_message(&sender_username, &message, is_server_message, &username);
            messages.push(formatted_message);
            log_message(&log_file, &format!("[DEBUG] Received message from {}: {}", sender_username, message)).await;
	}
    }
    
    // Clean up terminal after exit
    log_message(&log_file, "[DEBUG] Cleaning up terminal").await;
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}
