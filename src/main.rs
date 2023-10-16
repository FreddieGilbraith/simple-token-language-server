use ropey::Rope;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs::File;
use std::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

#[derive(Debug, Hash)]
struct Id {
    id: u64,
}

#[derive(Debug)]
struct State {
    files: HashMap<Url, Rope>,
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

impl State {
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
        }
    }

    pub fn open_file(&mut self, url: Url) -> std::io::Result<()> {
        let text = ropey::Rope::from_reader(File::open(url.path())?)?;

        self.files.insert(url, text);

        Ok(())
    }

    pub fn apply_change(&mut self, url: &Url, change: &TextDocumentContentChangeEvent) -> () {
        let rope = self.files.get_mut(url);

        match (change.range, rope) {
            (Some(Range { start, end }), Some(rope)) => {
                let start_index = rope.line_to_char(start.line as usize) + start.character as usize;
                let end_index = rope.line_to_char(end.line as usize) + (end.character as usize);

                rope.remove(start_index..end_index);
                rope.insert(start_index, change.text.as_str());

                ()
            }

            (None, _) => {
                let text = Rope::from_str(change.text.as_str());
                self.files.insert(url.clone(), text);
                ()
            }

            _ => (),
        }
    }

    #[allow(dead_code)]
    pub fn get_file_lines(&self, url: &Url) -> String {
        let mut v = Vec::new();
        self.files.get(&url).unwrap().write_to(&mut v).unwrap();
        String::from_utf8(v).unwrap()
    }

    pub fn get_completions(
        &self,
        query_from_url: &Url,
        position: &Position,
    ) -> Vec<CompletionItem> {
        let mut tokens = HashSet::new();
        let mut completion_items = vec![];
        let mut word_buffer = String::new();

        let query_line = usize::try_from(position.line).unwrap();

        for (url, rope) in self.files.iter() {
            let relative_url = url
                .make_relative(&query_from_url)
                .unwrap_or_else(|| url.to_string());

            for (line_number, line) in rope.lines().enumerate() {
                for (char) in line.chars() {
                    if line_number == query_line {
                        word_buffer.clear();
                        continue;
                    }

                    if char.is_alphanumeric() {
                        word_buffer.push(char)
                    } else {
                        if word_buffer.len() > 0 {
                            if tokens.insert(word_buffer.clone()) {
                                completion_items.push(CompletionItem::new_simple(
                                    word_buffer.clone(),
                                    relative_url.as_str().to_owned(),
                                ));
                            }

                            word_buffer.clear();
                        }
                    }
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

        let matches = self.state.read().unwrap().get_completions(&uri, &position);

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
