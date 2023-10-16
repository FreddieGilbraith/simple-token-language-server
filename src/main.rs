use ropey::Rope;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs::File;
use std::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

#[derive(Debug)]
struct SourceFile {
    rope: Rope,
    tokens: HashSet<String>,
}

#[derive(Debug)]
struct State {
    files: HashMap<Url, SourceFile>,
}

#[derive(Debug)]
struct Backend {
    client: Client,
    state: RwLock<State>,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            state: RwLock::new(State::new()),
        }
    }
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

    pub fn tokenize(&mut self, exclude_line: Option<usize>) -> () {
        let mut tokens: HashSet<String> = HashSet::new();
        let mut word_buffer = String::new();

        for (line_number, line) in self.rope.lines().enumerate() {
            if Some(line_number) == exclude_line {
                word_buffer.clear();
                continue;
            }

            for char in line.chars() {
                if char.is_alphanumeric() {
                    word_buffer.push(char)
                } else {
                    if word_buffer.len() > 0 {
                        tokens.insert(word_buffer.clone());

                        word_buffer.clear();
                    }
                }
            }
        }

        self.tokens = tokens;
    }
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
            for token in current_src_file.tokens.iter() {
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

            for token in src_file.tokens.iter() {
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

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                completion_provider: Some(CompletionOptions::default()),
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::INCREMENTAL,
                )),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "server initialized!")
            .await;
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        self.client
            .show_message(MessageType::INFO, "completion".to_string())
            .await;

        let TextDocumentPositionParams {
            position,
            text_document: TextDocumentIdentifier { uri },
        } = params.text_document_position;

        let matches = self.state.write().unwrap().get_completions(&uri, &position);

        Ok(Some(CompletionResponse::Array(matches)))
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) -> () {
        self.client
            .show_message(MessageType::INFO, "did_open".to_string())
            .await;

        let uri = params.text_document.uri;

        if uri.scheme() == "file" {
            self.client
                .show_message(MessageType::INFO, "is_file".to_string())
                .await;

            let _ = self.state.write().unwrap().open_file(uri);
        }

        ();
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        self.client
            .show_message(MessageType::INFO, "did_change".to_string())
            .await;

        let uri = params.text_document.uri;
        for change in params.content_changes {
            self.state.write().unwrap().apply_change(&uri, &change);
        }

        // let lines = self.state.read().unwrap().get_file_lines(&uri);

        // self.client.show_message(MessageType::INFO, &lines).await;

        ()
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend::new(client));
    Server::new(stdin, stdout, socket).serve(service).await;
}
