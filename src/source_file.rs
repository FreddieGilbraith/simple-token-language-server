use ropey::Rope;
use std::collections::HashSet;
use std::fs::File;
use tower_lsp::lsp_types::*;

#[derive(Debug)]
pub struct SourceFile {
    rope: Rope,
    tokens: HashSet<String>,
}

impl SourceFile {
    pub fn new(url: &Url) -> std::io::Result<Self> {
        let rope = ropey::Rope::from_reader(File::open(url.path())?)?;
        let tokens = HashSet::new();

        let mut me = Self { rope, tokens };
        me.tokenize(None);

        Ok(me)
    }

    pub fn apply_change(&mut self, change: &TextDocumentContentChangeEvent) -> () {
        match change.range {
            Some(Range { start, end }) => {
                let start_index =
                    self.rope.line_to_char(start.line as usize) + start.character as usize;
                let end_index =
                    self.rope.line_to_char(end.line as usize) + (end.character as usize);

                self.rope.remove(start_index..end_index);
                self.rope.insert(start_index, change.text.as_str());
            }

            None => {
                self.rope = ropey::Rope::from_str(change.text.as_str());
            }
        }

        self.tokenize(None)
    }

    pub fn words(
        &self,
        exclude_line: Option<usize>,
    ) -> Vec<(std::string::String, Position, Position)> {
        let mut words = Vec::new();
        let mut word_buffer = String::new();

        for (line_number, line) in self.rope.lines().enumerate() {
            if Some(line_number) == exclude_line {
                word_buffer.clear();
                continue;
            }

            for (char_number, char) in line.chars().enumerate() {
                if char.is_alphanumeric() {
                    word_buffer.push(char)
                } else {
                    if word_buffer.len() > 0 {
                        words.push((
                            word_buffer.clone(),
                            Position::new(
                                line_number as u32,
                                (char_number - word_buffer.len()) as u32,
                            ),
                            Position::new(line_number as u32, char_number as u32),
                        ));

                        word_buffer.clear();
                    }
                }
            }
        }

        words
    }

    pub fn tokenize(&mut self, exclude_line: Option<usize>) -> () {
        self.tokens = self
            .words(exclude_line)
            .iter()
            .map(|(word, _, _)| word.into())
            .collect();
    }

    pub fn tokens(&self) -> std::collections::hash_set::Iter<'_, std::string::String> {
        self.tokens.iter()
    }
}
