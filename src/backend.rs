use crate::state::State;
use std::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

#[derive(Debug)]
pub struct Backend {
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
