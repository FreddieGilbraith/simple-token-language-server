use crate::spell::Spell;
use crate::state::State;
use std::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

#[derive(Debug)]
pub struct Backend {
    client: Client,
    spell: Option<Spell>,

    state: RwLock<State>,
}

impl Backend {
    pub fn new(client: Client, spell: Option<Spell>) -> Self {
        Self {
            client,
            spell,
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
        let TextDocumentPositionParams {
            position,
            text_document: TextDocumentIdentifier { uri },
        } = params.text_document_position;

        let matches = self.state.write().unwrap().get_completions(&uri, &position);

        Ok(Some(CompletionResponse::Array(matches)))
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) -> () {
        let uri = params.text_document.uri;

        if uri.scheme() == "file" {
            let _ = self.state.write().unwrap().open_file(uri);
        }

        ();
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        for change in params.content_changes {
            self.state.write().unwrap().apply_change(&uri, &change);
        }

        if let Some(spell) = &self.spell {
            let mut spelling_errors = vec![];

            for (word, start, end) in self.state.read().unwrap().words(&uri, None).iter() {
                if !spell.is_valid(&word.to_lowercase()) {
                    spelling_errors.push(Diagnostic::new(
                        Range::new(start.clone(), end.clone()),
                        Some(DiagnosticSeverity::ERROR),
                        None,
                        Some("spell".into()),
                        word.clone(),
                        None,
                        None,
                    ))
                }
            }

            if spelling_errors.len() > 0 {
                self.client
                    .publish_diagnostics(uri, spelling_errors, None)
                    .await;
            }
        }

        ()
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}
