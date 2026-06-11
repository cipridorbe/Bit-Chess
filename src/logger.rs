use std::{fs::{File, OpenOptions}, io::{BufWriter, Write}, sync::Mutex};
use once_cell::sync::Lazy;

static LOG: Lazy<Mutex<Option<BufWriter<File>>>> = Lazy::new(|| Mutex::new(None));

pub fn init(path: &str) {
    let file = OpenOptions::new().create(true).truncate(true).write(true).open(path)
        .expect("failed to open log file");
    *LOG.lock().unwrap() = Some(BufWriter::new(file));
}

pub fn write(msg: &str) {
    if let Ok(mut guard) = LOG.lock() {
        if let Some(ref mut w) = *guard {
            let _ = writeln!(w, "{}", msg);
            let _ = w.flush();
        }
    }
}

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => { $crate::logger::write(&format!($($arg)*)) };
}
