use base64::{engine::general_purpose::STANDARD, Engine};
use std::fs::OpenOptions;
use std::io::{self, Write};
use zeroize::Zeroizing;

pub fn copy(text: &str) -> io::Result<()> {
    if text.bytes().any(|b| b < 0x20 || b == 0x7f) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "passphrase contains control characters unsafe for OSC 52",
        ));
    }
    let mut tty = OpenOptions::new().write(true).open("/dev/tty")?;
    let encoded = Zeroizing::new(STANDARD.encode(text));
    let osc52 = Zeroizing::new(format!("\x1b]52;c;{}\x07", encoded.as_str()));
    tty.write_all(osc52.as_bytes())
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

    #[test]
    fn rejects_control_characters() {
        assert!(copy("\x07").is_err());
        assert!(copy("word\x1bword").is_err());
        assert!(copy("\x7f").is_err());
    }
}
