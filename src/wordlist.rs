use std::sync::OnceLock;

const RAW: &str = include_str!("wordlist.txt");

static WORDS: OnceLock<Vec<&'static str>> = OnceLock::new();

pub fn load() -> &'static [&'static str] {
    WORDS.get_or_init(|| {
        RAW.lines()
            .filter_map(|line| line.split_once('\t').map(|(_, word)| word))
            .collect()
    })
}

#[cfg(test)]
mod tests {
    use super::load;
    use std::collections::HashSet;

    #[test]
    fn exactly_7776_words() {
        assert_eq!(load().len(), 7776);
    }

    #[test]
    fn first_and_last_words() {
        let words = load();
        assert_eq!(words[0], "abacus");
        assert_eq!(words[words.len() - 1], "zoom");
    }

    #[test]
    fn no_duplicates() {
        let words = load();
        let unique: HashSet<_> = words.iter().collect();
        assert_eq!(
            unique.len(),
            words.len(),
            "wordlist contains duplicate entries"
        );
    }

    #[test]
    fn all_lowercase() {
        for &word in load() {
            assert_eq!(
                word,
                word.to_lowercase(),
                "word '{word}' is not all lowercase"
            );
        }
    }

    #[test]
    fn no_empty_words() {
        for &word in load() {
            assert!(!word.is_empty(), "wordlist contains an empty entry");
        }
    }

    #[test]
    fn only_lowercase_alpha_and_hyphens() {
        for &word in load() {
            assert!(
                word.chars().all(|c| c.is_ascii_lowercase() || c == '-'),
                "word '{word}' contains unexpected characters"
            );
        }
    }
}
