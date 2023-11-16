use crate::source_file::SourceFile;
use std::collections::HashMap;
use std::collections::HashSet;
use tower_lsp::lsp_types::*;

#[derive(Debug)]
pub struct State {
    files: HashMap<Url, SourceFile>,
}

impl State {
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
        }
    }

    pub fn open_file(&mut self, url: Url) -> std::io::Result<()> {
        let src_file = SourceFile::new(&url)?;
        self.files.insert(url, src_file);

        Ok(())
    }

    pub fn apply_change(&mut self, url: &Url, change: &TextDocumentContentChangeEvent) -> () {
        let src_file = self.files.get_mut(url);

        if let Some(src_file) = src_file {
            src_file.apply_change(&change);
        }
    }

    pub fn get_completions(
        &mut self,
        query_from_url: &Url,
        position: &Position,
    ) -> Vec<CompletionItem> {
        let mut tokens = HashSet::new();
        let mut completion_items = vec![];

        let query_line = usize::try_from(position.line).unwrap();

        if let Some(current_src_file) = self.files.get_mut(query_from_url) {
            current_src_file.tokenize(Some(query_line));
        }

        if let Some(current_src_file) = self.files.get(query_from_url) {
            for token in current_src_file.tokens() {
                if tokens.insert(token) {
                    completion_items
                        .push(CompletionItem::new_simple(token.clone(), ".".to_owned()));
                }
            }
        }

        for (url, src_file) in self.files.iter() {
            if url == query_from_url {
                continue;
            }

            let relative_url = url
                .make_relative(&query_from_url)
                .unwrap_or_else(|| url.to_string());

            for token in src_file.tokens() {
                if tokens.insert(token) {
                    completion_items.push(CompletionItem::new_simple(
                        token.clone(),
                        relative_url.as_str().to_owned(),
                    ));
                }
            }
        }

        completion_items
    }
}
