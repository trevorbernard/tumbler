use std::fs::File;
use std::io::{self, Read, Write};

enum Inner {
    Device(File),
    Dice { next_word: usize, total_words: usize },
}

pub struct EntropySource {
    inner: Inner,
}

impl EntropySource {
    pub fn open(path: &str) -> io::Result<Self> {
        let device = File::open(path).map_err(|e| {
            io::Error::new(e.kind(), format!("cannot open entropy device '{}': {}", path, e))
        })?;
        Ok(Self { inner: Inner::Device(device) })
    }

    pub fn dice(total_words: usize) -> Self {
        Self { inner: Inner::Dice { next_word: 0, total_words } }
    }

    pub fn next_index(&mut self, list_len: usize) -> io::Result<usize> {
        match &mut self.inner {
            Inner::Device(dev) => sample(dev, list_len),
            Inner::Dice { next_word, total_words } => {
                const DICE_COMBINATIONS: usize = 7776;
                if list_len != DICE_COMBINATIONS {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!("dice mode requires a {DICE_COMBINATIONS}-word list, but wordlist has {list_len} words"),
                    ));
                }
                *next_word += 1;
                let word_num = *next_word;
                let total = *total_words;
                read_dice_roll(word_num, total)
            }
        }
    }
}

fn read_dice_roll(word_num: usize, total_words: usize) -> io::Result<usize> {
    let stderr = io::stderr();
    loop {
        {
            let mut err = stderr.lock();
            write!(err, "Word {word_num}/{total_words} — roll 5 dice (1-6): ")?;
            err.flush()?;
        }

        let mut line = String::new();
        let n = io::stdin().read_line(&mut line)?;
        if n == 0 {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "stdin closed"));
        }

        match parse_dice_roll(&line) {
            Some(idx) => return Ok(idx),
            None => eprintln!("  enter exactly 5 digits, each 1-6 (e.g. 25341)"),
        }
    }
}

/// Parse a dice roll string into a wordlist index.
///
/// Accepts 5 digits (1–6) with optional whitespace between them.
/// Returns None if the input is invalid.
pub(crate) fn parse_dice_roll(s: &str) -> Option<usize> {
    let digits: Vec<u8> = s
        .chars()
        .filter(|c| !c.is_whitespace())
        .map(|c| c as u8)
        .collect();
    if digits.len() == 5 && digits.iter().all(|&b| (b'1'..=b'6').contains(&b)) {
        Some(digits.iter().fold(0usize, |acc, &b| acc * 6 + (b - b'1') as usize))
    } else {
        None
    }
}

/// Uniformly distributed index in [0, list_len) via rejection sampling over u64.
///
/// Rejection probability = (2^64 mod list_len) / 2^64.
/// For a 7776-word list this is 2624/2^64 ≈ 1.4e-16; the loop body
/// executes exactly once in any practical lifetime of the universe.
pub(crate) fn sample<R: Read>(source: &mut R, list_len: usize) -> io::Result<usize> {
    assert!(list_len > 0, "list_len must be > 0");
    let n = list_len as u64;
    // threshold == 2^64 mod n; reject values in [0, threshold) to avoid modulo bias.
    let threshold = n.wrapping_neg() % n;
    loop {
        let mut buf = [0u8; 8];
        source.read_exact(&mut buf)?;
        let v = u64::from_le_bytes(buf);
        if v >= threshold {
            return Ok((v % n) as usize);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_dice_roll, sample};
    use std::io::Cursor;

    fn cursor(values: &[u64]) -> Cursor<Vec<u8>> {
        Cursor::new(values.iter().flat_map(|v| v.to_le_bytes()).collect())
    }

    #[test]
    fn dice_all_ones_is_index_zero() {
        assert_eq!(parse_dice_roll("11111"), Some(0));
    }

    #[test]
    fn dice_all_sixes_is_last_index() {
        assert_eq!(parse_dice_roll("66666"), Some(7775));
    }

    #[test]
    fn dice_spaces_accepted() {
        // "1 2 3 4 5" → digits [1,2,3,4,5] → (0)*1296 + (1)*216 + (2)*36 + (3)*6 + (4) = 310
        assert_eq!(parse_dice_roll("1 2 3 4 5"), Some(310));
    }

    #[test]
    fn dice_rejects_zero() {
        assert_eq!(parse_dice_roll("01234"), None);
    }

    #[test]
    fn dice_rejects_seven() {
        assert_eq!(parse_dice_roll("12375"), None);
    }

    #[test]
    fn dice_rejects_too_short() {
        assert_eq!(parse_dice_roll("1234"), None);
    }

    #[test]
    fn dice_rejects_too_long() {
        assert_eq!(parse_dice_roll("123456"), None);
    }

    #[test]
    fn dice_index_calculation() {
        // 21345: (2-1)*1296 + (1-1)*216 + (3-1)*36 + (4-1)*6 + (5-1) = 1296 + 0 + 72 + 18 + 4 = 1390
        assert_eq!(parse_dice_roll("21345"), Some(1390));
    }

    #[test]
    fn known_value_maps_correctly() {
        // 14 % 7 == 0, 15 % 7 == 1, 20 % 7 == 6
        let mut src = cursor(&[14, 15, 20]);
        assert_eq!(sample(&mut src, 7).unwrap(), 0);
        assert_eq!(sample(&mut src, 7).unwrap(), 1);
        assert_eq!(sample(&mut src, 7).unwrap(), 6);
    }

    #[test]
    fn rejection_sampling_skips_invalid_value() {
        // For list_len = 7: threshold = 7u64.wrapping_neg() % 7 == 2.
        // Values in [0, 2) are rejected, so both 0 and 1 are rejected; 14 is accepted.
        let mut src = cursor(&[0, 14]);
        assert_eq!(sample(&mut src, 7).unwrap(), 0); // 14 % 7 == 0
    }

    #[test]
    fn all_indices_in_range() {
        // Draw 10_000 samples from /dev/urandom and verify every result is valid.
        let list_len = 7776;
        let mut rng = std::fs::File::open("/dev/urandom").unwrap();
        for _ in 0..10_000 {
            let idx = sample(&mut rng, list_len).unwrap();
            assert!(idx < list_len, "index {idx} out of range [0, {list_len})");
        }
    }

    #[test]
    fn uniform_distribution() {
        // Generate 50_000 samples over a small list and verify every item
        // appears at least once. P(any item missing) is negligible at this count.
        let list_len = 10usize;
        let mut counts = vec![0u32; list_len];
        let mut rng = std::fs::File::open("/dev/urandom").unwrap();
        for _ in 0..50_000 {
            counts[sample(&mut rng, list_len).unwrap()] += 1;
        }
        for (i, &count) in counts.iter().enumerate() {
            assert!(count > 0, "index {i} never selected in 50_000 draws");
        }
    }

    #[test]
    fn open_nonexistent_device_gives_clear_error() {
        match super::EntropySource::open("/nonexistent/device") {
            Ok(_) => panic!("expected an error opening a nonexistent device"),
            Err(e) => assert!(
                e.to_string().contains("/nonexistent/device"),
                "error should name the device: {e}"
            ),
        }
    }

    #[test]
    fn exhausted_source_returns_error() {
        let mut src = cursor(&[14]);
        assert!(sample(&mut src, 7).is_ok());
        // No more bytes — next call must fail, not hang or panic.
        assert!(sample(&mut src, 7).is_err());
    }
}
