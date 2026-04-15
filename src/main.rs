use argh::FromArgs;
use std::io;
use zeroize::Zeroize;

mod clipboard;
mod entropy;
mod wordlist;

#[derive(FromArgs)]
/// Generate a diceware passphrase using a hardware-selectable entropy source.
///
/// The passphrase is copied to your clipboard by default so it never appears
/// in terminal output or shell history. Use --print to write to stdout instead.
struct Args {
    /// number of words to generate [default: 6]
    #[argh(option, short = 'n', default = "6")]
    words: usize,

    /// target entropy in bits; computes word count automatically (overrides --words)
    #[argh(option, short = 'b')]
    bits: Option<f64>,

    /// number of passphrases to generate [default: 1]
    #[argh(option, short = 'c', default = "1")]
    count: usize,

    /// entropy source device [default: /dev/urandom]
    ///
    /// Use /dev/hwrng to read directly from a hardware TRNG. Note that raw
    /// hardware RNG output is unwhitened; prefer /dev/urandom unless you have
    /// a specific reason to bypass the kernel CSPRNG.
    #[argh(option, short = 'd', default = "String::from(\"/dev/urandom\")")]
    device: String,

    /// word separator [default: ""]
    #[argh(option, short = 's', default = "String::new()")]
    separator: String,

    /// do not capitalize words
    #[argh(switch)]
    no_capitalize: bool,

    /// use physical dice rolls as the entropy source
    ///
    /// Prompts you to roll 5 dice per word. Each set of 5 rolls (digits 1-6)
    /// selects one word from the 7776-word list with no computer randomness.
    #[argh(switch)]
    dice: bool,

    /// print passphrase to stdout instead of copying to clipboard
    #[argh(switch, short = 'p')]
    print: bool,

    /// show entropy statistics
    #[argh(switch, short = 'e')]
    entropy: bool,
}

fn main() -> io::Result<()> {
    let args: Args = argh::from_env();

    if args.count == 0 {
        eprintln!("error: --count must be at least 1");
        std::process::exit(1);
    }

    if args.dice && args.device != "/dev/urandom" {
        eprintln!("error: --dice and --device are mutually exclusive");
        std::process::exit(1);
    }

    let words = wordlist::load();
    let bits_per_word = (words.len() as f64).log2();

    let word_count = if let Some(b) = args.bits {
        if b <= 0.0 {
            eprintln!("error: --bits must be > 0");
            std::process::exit(1);
        }
        ((b / bits_per_word).ceil() as usize).max(1)
    } else {
        if args.words == 0 {
            eprintln!("error: --words must be at least 1");
            std::process::exit(1);
        }
        args.words
    };

    if args.entropy {
        let total_bits = bits_per_word * word_count as f64;
        eprintln!(
            "{} words × {:.3} bits/word = {:.1} bits  (wordlist: {} words)",
            word_count,
            bits_per_word,
            total_bits,
            words.len(),
        );
    }

    let print = args.print || args.count > 1;

    for _ in 0..args.count {
        let mut passphrase = generate_passphrase(&words, word_count, &args)?;

        if print {
            println!("{}", passphrase);
        } else {
            match clipboard::copy(&passphrase) {
                Ok(clipboard::Destination::Clipboard) => eprintln!("Copied to clipboard."),
                Ok(clipboard::Destination::Stdout) => {}
                Err(e) => {
                    eprintln!("warning: clipboard write failed ({}), printing instead", e);
                    println!("{}", passphrase);
                }
            }
        }

        passphrase.zeroize();
    }

    Ok(())
}

fn generate_passphrase(words: &[&str], word_count: usize, args: &Args) -> io::Result<String> {
    let mut rng = if args.dice {
        entropy::EntropySource::dice(word_count)
    } else {
        entropy::EntropySource::open(&args.device)?
    };
    let selected: Vec<String> = (0..word_count)
        .map(|_| {
            rng.next_index(words.len()).map(|i| {
                if args.no_capitalize {
                    words[i].to_string()
                } else {
                    capitalize(words[i])
                }
            })
        })
        .collect::<io::Result<_>>()?;
    Ok(selected.join(&args.separator))
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

#[cfg(test)]
mod tests {
    use super::{capitalize, generate_passphrase, Args};

    fn args(words: usize, separator: &str, no_capitalize: bool) -> Args {
        Args {
            words,
            bits: None,
            count: 1,
            device: "/dev/urandom".to_string(),
            separator: separator.to_string(),
            no_capitalize,
            dice: false,
            print: false,
            entropy: false,
        }
    }

    #[test]
    fn capitalize_regular_word() {
        assert_eq!(capitalize("hello"), "Hello");
    }

    #[test]
    fn capitalize_already_upper() {
        assert_eq!(capitalize("Hello"), "Hello");
    }

    #[test]
    fn capitalize_single_char() {
        assert_eq!(capitalize("a"), "A");
    }

    #[test]
    fn capitalize_empty() {
        assert_eq!(capitalize(""), "");
    }

    #[test]
    fn generate_word_count() {
        let words = crate::wordlist::load();
        for n in [1, 4, 6, 8] {
            let phrase = generate_passphrase(&words, n, &args(n, " ", false)).unwrap();
            assert_eq!(phrase.split(' ').count(), n, "expected {n} words");
        }
    }

    #[test]
    fn generate_capitalized_by_default() {
        let words = crate::wordlist::load();
        let phrase = generate_passphrase(&words, 6, &args(6, " ", false)).unwrap();
        for word in phrase.split(' ') {
            let first = word.chars().next().unwrap();
            assert!(first.is_uppercase(), "'{word}' should start with uppercase");
        }
    }

    #[test]
    fn generate_no_capitalize() {
        let words = crate::wordlist::load();
        let phrase = generate_passphrase(&words, 6, &args(6, " ", true)).unwrap();
        for word in phrase.split(' ') {
            assert_eq!(word, word.to_lowercase(), "'{word}' should be all lowercase");
        }
    }

    #[test]
    fn generate_respects_separator() {
        let words = crate::wordlist::load();
        let phrase = generate_passphrase(&words, 4, &args(4, "-", true)).unwrap();
        assert_eq!(phrase.matches('-').count(), 3, "4 words need 3 hyphens");
    }

    #[test]
    fn generate_default_no_separator() {
        let words = crate::wordlist::load();
        let phrase = generate_passphrase(&words, 4, &args(4, "", false)).unwrap();
        assert!(!phrase.contains(' '), "default separator should be empty");
        assert!(
            phrase.chars().all(|c| c.is_ascii_alphabetic() || c == '-'),
            "passphrase should be alpha/hyphen only with empty separator"
        );
    }

    #[test]
    fn generate_bad_device_returns_error() {
        let words = crate::wordlist::load();
        let a = Args {
            words: 1,
            bits: None,
            count: 1,
            device: "/nonexistent/device".to_string(),
            separator: String::new(),
            no_capitalize: false,
            dice: false,
            print: false,
            entropy: false,
        };
        assert!(generate_passphrase(&words, 1, &a).is_err());
    }
}
