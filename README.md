# **TermTalk: A Terminal-Based Chat Application in Rust**

This repository contains a **terminal-based chat application** built in Rust, leveraging asynchronous programming and multithreading for real-time communication. The application consists of a **server** and a **client**, allowing multiple users to connect and chat in a shared terminal environment.

---

## **Features**
- **Real-time messaging**: Send and receive messages instantly with other connected users.
- **Asynchronous I/O**: Built using `tokio` for efficient handling of multiple clients.
- **Terminal UI**: Clean and intuitive terminal interface powered by `tui` and `crossterm`.
- **Mention highlighting**: Mentions (e.g., `@username`) are highlighted for better visibility.
- **Ping-Pong mechanism**: Ensures clients remain connected to the server.

---

## **How It Works**
The application is divided into two components:
1. **Server**: Manages client connections, broadcasts messages, and handles disconnections.
2. **Client**: Connects to the server, sends messages, and displays the chat interface.

Messages are broadcast to all connected clients in real-time, with timestamps and colored usernames for clarity.

---

## **System Requirements**
- **Rust and Cargo**: Ensure you have Rust installed. If not, install it from [rustup.rs](https://rustup.rs/).
- **Unix-like environment**: Tested on **Fedora 40**, but should work on Linux, macOS, and other Unix-like systems.

---

## **How to Run**

### 1. Clone the Repository
```bash
git clone https://github.com/Peter-L-SVK/termtalk.git
cd termtalk
```

### 2. Build the Project
```bash
cargo build --release
```

### 3. Start the Server
Run the server on your machine:
```bash
./target/release/server
```
The server will start listening on `127.0.0.1:8080`.

### 4. Start the Client
In a new terminal, start the client:
```bash
./target/release/client
```
You will be prompted to enter a username. Once connected, you can start chatting!

---

## **Customization**

### Change the server IP/port
Modify the server address in `client.rs`:
```rust
TcpStream::connect("127.0.0.1:8080").await
```

### Modify the terminal UI
Adjust the layout and styling in `client.rs` using the `tui` crate.

---

## **Dependencies**
This project uses the following Rust crates:

- **tokio**: Asynchronous runtime for networking.
- **tui**: Terminal user interface library.
- **crossterm**: Cross-platform terminal handling.
- **chrono**: Timestamp formatting.
- **colored**: Colored text output.
