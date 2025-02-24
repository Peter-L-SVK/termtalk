use chrono::Local;
use tokio::net::TcpStream;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tokio::sync::broadcast;
use tokio::sync::Mutex; // Use tokio::sync::Mutex for async compatibility
use colored::*;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io::{self, Write};
use std::fs::OpenOptions;
use std::sync::Arc;
use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders, Paragraph},
    text::{Text, Span, Spans}, // Import Spans as well
    style::{Style, Color},
    Terminal,
};
use tokio::time::{self, Duration}; // Add tokio::time for interval
use unicode_width::UnicodeWidthStr; // Add this crate to measure visible width
use strip_ansi_escapes::strip as strip_ansi_escapes; // Add this crate to strip ANSI escape sequences

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Set up terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    
    // Create a log file
    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("client.log")?;

    // Wrap the log file in an Arc<Mutex<>> for shared ownership
    let log_file = Arc::new(Mutex::new(log_file));

    // Helper function to log debug messages to the log file
    async fn log_debug(log_file: &Arc<Mutex<std::fs::File>>, message: &str) {
        let mut file = log_file.lock().await; // Use .await for tokio::sync::Mutex
        if writeln!(&mut *file, "{}", message).is_err() {
            eprintln!("Failed to write to log file: {}", message);
        }
    }

    // Log terminal initialization
    let debug_message = format!("[DEBUG] Terminal initialized successfully");
    log_debug(&log_file, &debug_message).await; // Log to file

    // Create a broadcast channel for message broadcasting
    let (sender, mut receiver) = broadcast::channel(32);

    // Connect to the server
    let debug_message = format!("[DEBUG] Connecting to server...");
    log_debug(&log_file, &debug_message).await; // Log to file
    let stream = match TcpStream::connect("127.0.0.1:8080").await {
        Ok(stream) => {
            let debug_message = format!("[DEBUG] Connected to server");
            log_debug(&log_file, &debug_message).await; // Log to file
            stream
        }
        Err(_) => {
            let debug_message = format!("[DEBUG] Failed to connect to the server");
            log_debug(&log_file, &debug_message).await; // Log to file
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
        let debug_message = format!("[DEBUG] Failed to read token from server");
        log_debug(&log_file, &debug_message).await; // Log to file
        return Ok(());
    }
    let client_token = token_message.trim().split_whitespace().last().unwrap_or("unknown").to_string();
    let debug_message = format!("[DEBUG] Client token: {}", client_token);
    log_debug(&log_file, &debug_message).await; // Log to file

    // Prompt the client for a username
    let mut username = String::new();
    let mut error_message = String::new(); // Store error messages
    loop {
        terminal.draw(|f| {
            let size = f.size();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(
                    [
                        Constraint::Percentage(80), // Username input area
                        Constraint::Percentage(20), // Error message area
                    ]
                    .as_ref(),
                )
                .split(size);

            // Render username input area
            let input_block = Paragraph::new(format!("Enter your username: {}", username))
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(input_block, chunks[0]);

            // Render error message area
            let error_block = Paragraph::new(Text::from(Spans::from(vec![
                Span::styled(
                    error_message.clone(),
                    Style::default().fg(Color::Red), // Apply red color
                )
            ])))
            .block(Block::default().borders(Borders::ALL));
            f.render_widget(error_block, chunks[1]);
        })?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Enter => {
                    if username.trim().is_empty() {
                        // Show error message if username is empty
                        error_message = "Error: Username cannot be empty!".to_string();
                    } else {
			// Send the username to the server
			{
			    let mut write_stream = write_stream.lock().await;
			    if write_stream.write_all(format!("{}\n", username).as_bytes()).await.is_err() {
				let debug_message = format!("[DEBUG] Failed to send username to server");
				log_debug(&log_file, &debug_message).await; // Log to file
				return Ok(());
			    }
			}
			
			// Read the server's response
			let mut response = String::new();
			if reader.read_line(&mut response).await.is_err() {
			    let debug_message = format!("[DEBUG] Failed to read server response");
			    log_debug(&log_file, &debug_message).await; // Log to file
			    return Ok(());
			}
			
			// Log the server's response
			let debug_message = format!("[DEBUG] Server response: {}", response.trim());
			log_debug(&log_file, &debug_message).await; // Log to file
			
			// Filter out the "Enter your username: " part from the response
			let response = response.trim().strip_prefix("Enter your username: ").unwrap_or(response.trim());
			
			// Check if the server rejected the username
			if response == "ERROR: Username is already taken. Please choose a different one." {
			    // Show error message if username is already taken
			    error_message = response.to_string();
			    username.clear(); // Clear the username input
			    continue; // Go back to the username input loop
			}
			
			// Check if the server accepted the username
			if response == "SUCCESS: Username accepted." {
			    break; // Proceed to the chat state
			}
			
			// If the response is unexpected, show an error and retry
			error_message = "Unexpected server response. Please try again.".to_string();
			username.clear(); // Clear the username input
			continue; // Go back to the username input loop
                    }
                }
                KeyCode::Backspace => {
                    username.pop();
                    error_message.clear(); // Clear error message when user edits input
                }
                KeyCode::Char(c) => {
                    username.push(c);
                    error_message.clear(); // Clear error message when user edits input
                }
                _ => {}
            }
        }
    }

    // Transition to chat state
    let debug_message = format!("[DEBUG] Transitioning to chat state");
    log_debug(&log_file, &debug_message).await; // Log to file

    // Clear the terminal and render the chat UI
    terminal.clear()?; // Clear the terminal to remove the login screen

    // Spawn a task to handle the client
    let sender_clone = sender.clone();
    let _sender_main = sender.clone(); // Clone again for main loop
    let username_clone = username.clone();
    let log_file_clone = Arc::clone(&log_file); // Clone Arc for the task
    let client_token_clone = client_token.clone(); // Clone client token for the task
    let write_stream_clone = Arc::clone(&write_stream); // Clone write_stream for the task
    tokio::spawn(async move {
        let debug_message = format!("[DEBUG] Spawning client handler task for token {}", client_token_clone);
        log_debug(&log_file_clone, &debug_message).await; // Log to file
        let mut input = String::new();
        loop {
            input.clear();
            if reader.read_line(&mut input).await.is_err() {
                let debug_message = format!("[DEBUG] Disconnected from server");
                log_debug(&log_file_clone, &debug_message).await; // Log to file
                break;
            }

            // Handle ping messages
            if input.trim() == "PING" {
                {
                    let mut write_stream = write_stream_clone.lock().await;
                    if write_stream.write_all(b"PONG\n").await.is_err() {
                        let debug_message = format!("[DEBUG] Failed to send pong to server");
                        log_debug(&log_file_clone, &debug_message).await; // Log to file
                        break;
                    }
                } // MutexGuard is dropped here
                continue;
            }

            // Handle regular messages
            let timestamp = Local::now().format("[%d.%m.%Y %H:%M]").to_string();
            let message = format!(
                "{} {}", // Remove the static username next to the date
                timestamp.black(), // Timestamp in black
                input.trim() // Message text
            );
            let debug_message = format!("[DEBUG] Broadcasting message from token {}: {}", client_token_clone, message);
            log_debug(&log_file_clone, &debug_message).await; // Log to file
            let _ = sender_clone.send((username_clone.clone(), input.trim().to_string())); // Broadcast the raw input
        }
        let debug_message = format!("[DEBUG] Client handler task for token {} exited", client_token_clone);
        log_debug(&log_file_clone, &debug_message).await; // Log to file
    });

    // Chat UI loop
    let mut messages: Vec<String> = Vec::new(); // Define messages storage
    let mut input_text = String::new(); // Store user input
    let mut scroll_offset: u16 = 0; // Track scroll position

    // Create a timer to refresh the UI every 0.1 seconds (faster refresh rate)
    let mut interval = time::interval(Duration::from_millis(100));

    loop {
        // Wait for the next tick or a key event
        tokio::select! {
            _ = interval.tick() => {
                // Refresh the message box only
                terminal.draw(|f| {
                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints(
                            [
                                Constraint::Percentage(90), // Message area
                                Constraint::Percentage(10), // Input area
                            ]
                            .as_ref(),
                        )
                        .split(f.size());

                    // Render messages
                    let message_block = Paragraph::new(messages.join("\n")) // Convert Vec<String> to a string
                        .block(Block::default().borders(Borders::ALL))
                        .scroll((scroll_offset, 0)); // Enable scrolling
                    f.render_widget(message_block, chunks[0]);

                    // Render input area
                    let input_block = Paragraph::new(format!("{}: {}", username, input_text)) // Add username prompt
                        .block(Block::default().borders(Borders::ALL));
                    f.render_widget(input_block, chunks[1]);
                })?;
            }
            event = async { event::read() } => {
                // Handle input and broadcast messages
                if let Event::Key(key) = event? {
                    match key.code {
                        KeyCode::Enter => {
                            if !input_text.is_empty() {
                                // Send the message to the server
                                {
                                    let mut write_stream = write_stream.lock().await;
                                    if write_stream.write_all(format!("{}\n", input_text.trim()).as_bytes()).await.is_err() {
                                        let debug_message = format!("[DEBUG] Failed to send message to server");
                                        log_debug(&log_file, &debug_message).await; // Log to file
                                        break;
                                    }
                                }

                                // Clear input text after sending the message
                                input_text.clear();
                            }
                        }
                        KeyCode::Backspace => {
                            input_text.pop();
                        }
                        KeyCode::Char(c) => {
                            input_text.push(c);
                        }
                        KeyCode::Up => {
                            // Scroll up
                            scroll_offset = scroll_offset.saturating_sub(1);
                        }
                        KeyCode::Down => {
                            // Scroll down
                            scroll_offset = scroll_offset.saturating_add(1);
                        }
                        KeyCode::Esc => break, // Exit the chat on Esc
                        _ => {}
                    }
                }
            }
        }

        // Receive messages from the broadcast channel
	while let Ok((sender_username, message)) = receiver.try_recv() {
	    // Filter out the "Enter your username:" line
	    if message.contains("Enter your username:") {
		continue; // Skip this message
	    }
	    
	    // Format and display the message
	    let timestamp = Local::now().format("[%d.%m.%Y %H:%M]").to_string();
	    
	    // Check if the message is from SERVER
	    let is_server_message = message.starts_with("SERVER:");
	    
	    // Helper function to calculate the visible width of a string (ignoring ANSI escape sequences)
	    let visible_width = |s: &str| -> usize {
		let stripped = strip_ansi_escapes(s).unwrap_or_else(|_| s.as_bytes().to_vec());
		let stripped_str = String::from_utf8_lossy(&stripped);
		stripped_str.width() // Measure the visible width
	    };
	    
	    // Combine the timestamp, colored username, and formatted content
	    let formatted_message = if is_server_message {
		// Format the entire message in magenta
		format!(
		    "{} {}", // Format: [timestamp] message
		    timestamp.black(), // Timestamp in black
		    message.magenta() // Entire message in magenta
		)
	    } else {
		// Regular user message
		let mut parts = message.splitn(2, ':');
		let message_username = parts.next().unwrap_or("").trim(); // Extract the username part
		let message_content = parts.next().unwrap_or("").trim(); // Extract the message content
		
		let colored_username = if message_username == username {
		    message_username.green().to_string() // Logged-in user's username is green
		} else {
		    message_username.blue().to_string() // Other users' usernames are blue
		};
		
		// Apply mention highlighting
		let formatted_content = if message_content.contains(&format!("@{}", username)) || message_content.contains("@all") {
		    // If the message contains a mention of the logged-in user or @all, highlight it in red and bold
		    message_content
			.split_whitespace()
			.map(|word| {
			    if word == &format!("@{}", username) || word == "@all" {
				word.red().bold().to_string() // Highlight @username and @all in red bold
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
		
		// Calculate the visible width of the username
		let username_width = visible_width(&colored_username);
		
		// Pad the username to a minimum width of 10 characters
		let padded_username = format!("{:width$}", colored_username, width = username_width.max(10));
		
		// Combine the timestamp, padded username, and formatted content
		format!(
		    "{} {}: {}", // Format: [timestamp] username: message
		    timestamp.black(), // Timestamp in black
		    padded_username, // Padded username
		    formatted_content // Formatted message content
		)
	    };
	    
	    // Add the formatted message to the UI
	    messages.push(formatted_message);
	    let debug_message = format!("[DEBUG] Received message from {}: {}", sender_username, message);
	    log_debug(&log_file, &debug_message).await; // Log to file
	}
    }
    
    // Clean up terminal after exit
    let debug_message = format!("[DEBUG] Cleaning up terminal");
    log_debug(&log_file, &debug_message).await; // Log to file
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}
