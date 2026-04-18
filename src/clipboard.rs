use base64::{engine::general_purpose::STANDARD, Engine};
use std::fs::OpenOptions;
use std::io::{self, Write};
use zeroize::Zeroizing;

pub enum Destination {
    Clipboard,
    Stdout,
}

pub fn copy(text: &str) -> io::Result<Destination> {
    match OpenOptions::new().write(true).open("/dev/tty") {
        Ok(mut tty) => {
            let encoded = Zeroizing::new(STANDARD.encode(text));
            let osc52 = Zeroizing::new(format!("\x1b]52;c;{}\x07", encoded.as_str()));
            tty.write_all(osc52.as_bytes())?;
            Ok(Destination::Clipboard)
        }
        Err(_) => {
            println!("{}", text);
            Ok(Destination::Stdout)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn osc52_sequence_format() {
        let text = "hello";
        let seq = format!("\x1b]52;c;{}\x07", STANDARD.encode(text));

        assert!(seq.starts_with("\x1b]52;c;"));
        assert!(seq.ends_with('\x07'));
    }
}
