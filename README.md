# **TermTalk: A Terminal-Based Chat Application in Rust**

**Version: 0.1**

This repository contains a simple **terminal-based chat application** built in Rust, leveraging asynchronous programming and multithreading for real-time communication. The application consists of a **server** and a **client**, allowing multiple users to connect and chat in a shared terminal environment. You can ran several separate terminal clients and emulate users talking. Created and tested on Fedora 40. The development is still ongoing and will continue for the time. This is a hobby project of mine and proof of concept rather than proper app. 

---

## **Screenshot**

Here is a screenshot of TermTalk in action(terminal app used: Terminator):

![TermTalk Chat Screenshot](example.png)

---

## **Features**
- **Build and run scripts for server and client**: Scripts will build apps using `cargo run` if project is built already, they will run only binaries.
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
- **Unix-like environment**: Tested on **Fedora 40**, but should work on other Linux distributions, macOS and FreeBSD.

---

## **How to Run**

### 1. Clone the Repository
```bash
git clone https://github.com/Peter-L-SVK/termtalk.git
cd termtalk
```

### 2. Build the Project
The included BASH scripts will also bulid the apps and run the binaries or build them straight away:
```bash
cargo build --release
```

### 3. Start the Server
You can run included BASH server script to run the server on your machine or:
```bash
./target/release/server
```
The server will start listening on `127.0.0.1:8080`.

### 4. Start the Client
In a new terminal you can run included BASH script for running client or start the client:
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

