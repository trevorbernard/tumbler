use sha2::{Digest, Sha256};

// SHA-256 of the EFF large wordlist (eff_large_wordlist.txt, 7776 entries).
// If this check fails, replace src/wordlist.txt with the file from:
//   https://www.eff.org/files/2016/07/18/eff_large_wordlist.txt
// and verify its hash independently before updating this constant.
const EXPECTED: &str = "addd35536511597a02fa0a9ff1e5284677b8883b83e986e43f15a3db996b903e";

fn main() {
    let bytes = std::fs::read("src/wordlist.txt").expect("src/wordlist.txt not found");

    let digest = Sha256::digest(&bytes);
    let actual: String = digest.iter().map(|b| format!("{b:02x}")).collect();

    if actual != EXPECTED {
        panic!(
            "wordlist checksum mismatch\n  expected: {}\n    actual: {}",
            EXPECTED, actual
        );
    }

    println!("cargo:rerun-if-changed=src/wordlist.txt");
}
