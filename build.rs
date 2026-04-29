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

    let version = env!("CARGO_PKG_VERSION");
    let sha = if version.contains("-dev") {
        println!("cargo:rerun-if-changed=.git/HEAD");
        // Prefer an injected value (e.g. from nix, where .git is absent)
        std::env::var("GIT_SHORT_SHA")
            .ok()
            .filter(|s| !s.is_empty())
            .or_else(|| {
                std::process::Command::new("git")
                    .args(["rev-parse", "--short", "HEAD"])
                    .output()
                    .ok()
                    .filter(|o| o.status.success())
                    .and_then(|o| String::from_utf8(o.stdout).ok())
                    .map(|s| s.trim().to_owned())
            })
            .unwrap_or_default()
    } else {
        String::new()
    };

    println!("cargo:rustc-env=GIT_SHORT_SHA={sha}");
}
