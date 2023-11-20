use std::collections::HashSet;
use std::error::Error;
use std::path::PathBuf;
use std::str;
use tokio::fs;

#[derive(Debug)]
pub struct Spell {
    words: HashSet<String>,
}

impl Spell {
    pub async fn new(dict: &PathBuf) -> Result<Self, Box<dyn Error>> {
        let contents = fs::read(dict).await?;

        let words = str::from_utf8(&contents)?
            .split('\n')
            .map(String::from)
            .collect();

        Ok(Self { words })
    }

    pub fn is_valid(&self, word: &str) -> bool {
        self.words.contains(word)
    }
}
