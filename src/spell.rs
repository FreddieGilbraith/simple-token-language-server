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
        if word.len() < 4 {
            return true;
        }

        if self.multiple_capitals.is_match(word) {
            return true;
        }

        let word = word.to_lowercase();

        if word.chars().nth(word.len() - 1) == Some('s') {
            let unpluraled = String::from(&word[0..word.len() - 1]);
            self.words.contains(&word) || self.words.contains(&unpluraled)
        } else {
            self.words.contains(&word)
        }
    }

    pub fn get_suggestions(&self, query: &str) -> Vec<(u32, String)> {
        use stringmetrics::levenshtein;

        let mut suggs = vec![];
        let thresh = ((query.len() as u32) as f32).sqrt() as u32;

        for word in self.words.iter() {
            let dist = levenshtein(query, word);
            if dist < thresh {
                suggs.push((dist, word.clone()))
            }
        }

        suggs.sort_by(|(a, _), (b, _)| a.partial_cmp(&b).unwrap());
        suggs
    }
}
