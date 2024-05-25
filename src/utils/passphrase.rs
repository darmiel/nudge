use std::borrow::Cow;
use std::fmt::{Display, Formatter};
use rand::{Rng, thread_rng};
use serde::{Deserialize, Serialize};
use crate::error::Result;

/// A passphrase generator that can generate passphrases
pub struct PassphraseGenerator(Vec<String>);

// A passphrase, e.g. "correct-horse-battery"
#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Clone, Serialize, Deserialize)]
pub struct Passphrase<'a>(pub Cow<'a, str>);

impl<'a> From<&'a str> for Passphrase<'a> {
    fn from(s: &'a str) -> Self {
        Passphrase(Cow::Borrowed(s))
    }
}

impl Display for Passphrase<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl PassphraseGenerator {
    const AVG_WORD_SIZE: usize = 5;

    /// Create a new PassphraseGenerator
    /// It will read the file `english-medium.txt` and store the lines in a vector
    pub fn new() -> Result<Self> {
        let content = include_str!("../english-medium.txt");
        let lines: Vec<String> = content.lines().map(str::to_owned).collect();
        Ok(PassphraseGenerator(lines))
    }

    /// Generate a passphrase with a given number of words
    ///
    pub fn generate_with_count(&self, word_count: usize) -> Option<Passphrase<'static>> {
        if word_count == 0 {
            return None;
        }

        let mut rng = thread_rng();
        let mut passphrase = String::with_capacity(
            word_count * Self::AVG_WORD_SIZE + word_count - 1
        );

        for i in 0..word_count {
            if i != 0 {
                passphrase.push('-');
            }
            let random_word = self.0.get(rng.gen_range(0..self.0.len()))?;
            passphrase.push_str(random_word);
        }

        Some(Passphrase(Cow::Owned(passphrase)))
    }

    /// Generate a passphrase with 3 words
    pub fn generate(&self) -> Option<Passphrase<'static>> {
        self.generate_with_count(3)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_passphrase_from_str() {
        let passphrase = Passphrase::from("example-passphrase");
        assert_eq!(passphrase.to_string(), "example-passphrase");
    }

    #[test]
    fn test_passphrase_generator_new() {
        let generator = PassphraseGenerator::new().unwrap();
        assert!(!generator.0.is_empty());
    }

    #[test]
    fn test_generate_with_zero_words() {
        let generator = PassphraseGenerator::new().unwrap();
        assert!(generator.generate_with_count(0).is_none());
    }

    #[test]
    fn test_generate_with_one_word() {
        let generator = PassphraseGenerator::new().unwrap();
        let passphrase = generator.generate_with_count(1).unwrap();
        assert!(!passphrase.to_string().contains('-'));
    }

    #[test]
    fn test_generate_with_multiple_words() {
        let generator = PassphraseGenerator::new().unwrap();
        let passphrase = generator.generate_with_count(3).unwrap();
        assert_eq!(passphrase.to_string().matches('-').count(), 2);
    }

    #[test]
    fn test_default_generate() {
        let generator = PassphraseGenerator::new().unwrap();
        let passphrase = generator.generate().unwrap();
        assert_eq!(passphrase.to_string().matches('-').count(), 2);
    }
}
