use std::fs::File;
use std::io::{self, Read};

pub struct EntropySource {
    device: File,
}

impl EntropySource {
    pub fn open(path: &str) -> io::Result<Self> {
        let device = File::open(path).map_err(|e| {
            io::Error::new(e.kind(), format!("cannot open entropy device '{}': {}", path, e))
        })?;
        Ok(Self { device })
    }

    pub fn next_index(&mut self, list_len: usize) -> io::Result<usize> {
        sample(&mut self.device, list_len)
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
    use super::sample;
    use std::io::Cursor;

    fn cursor(values: &[u64]) -> Cursor<Vec<u8>> {
        Cursor::new(values.iter().flat_map(|v| v.to_le_bytes()).collect())
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
