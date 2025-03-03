use std::io::Write;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::fs::File;

pub async fn log_message(log_file: &Arc<Mutex<File>>, message: &str) {
    let mut file = log_file.lock().await;
    if writeln!(&mut *file, "{}", message).is_err() {
        eprintln!("Failed to write to log file: {}", message);
    }
}
