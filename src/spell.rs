use regex::Regex;
use std::collections::HashSet;
use std::error::Error;
use std::path::PathBuf;
use std::str;
use tokio::fs;

#[derive(Debug)]
pub struct Spell {
    words: HashSet<String>,
    multiple_capitals: regex::Regex,
}

impl Spell {
    pub async fn new(dict: &PathBuf) -> Result<Self, Box<dyn Error>> {
        let multiple_capitals: regex::Regex = Regex::new(r".+[A-Z].+").unwrap();
        let contents = fs::read(dict).await?;

        let words = str::from_utf8(&contents)?
            .split('\n')
            .map(String::from)
            .collect();

        Ok(Self {
            words,
            multiple_capitals,
        })
    }

    pub fn is_valid(&self, word: &str) -> bool {
        if self.multiple_capitals.is_match(word) {
            return true;
        }

        self.words.contains(word)
    }
}
