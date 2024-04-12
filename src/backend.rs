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
        self.client
            .show_message(MessageType::LOG, "server 2initialized!")
            .await;

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                completion_provider: Some(CompletionOptions::default()),
                code_action_provider: Some(CodeActionProviderCapability::Options(
                    CodeActionOptions {
                        code_action_kinds: Some(vec![CodeActionKind::REFACTOR]),
                        resolve_provider: Some(true),
                        work_done_progress_options: WorkDoneProgressOptions::default(),
                    },
                )),
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::INCREMENTAL,
                )),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {}

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
                if !spell.is_valid(&word) {
                    spelling_errors.push(Diagnostic::new(
                        Range::new(start.clone(), end.clone()),
                        Some(DiagnosticSeverity::ERROR),
                        None,
                        Some("spell".into()),
                        format!("Invalid Spelling \"{}\"", &word),
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

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let uri = params.text_document.uri;

        let mut found_word = None;

        for (word, start, end) in self.state.read().unwrap().words(&uri, None).iter() {
            if start.line == params.range.start.line
                && start.character <= params.range.start.character
                && end.line == params.range.end.line
                && end.character >= params.range.end.character
            {
                if let Some(spell) = &self.spell {
                    if !spell.is_valid(&word) {
                        found_word = Some((word.clone(), start.clone(), end.clone()));
                    }
                }
                break;
            }
        }

        if let Some((found_word, start, end)) = found_word {
            let mut actions = if let Some(spell) = &self.spell {
                spell
                    .get_suggestions(&found_word)
                    .iter()
                    .map(|(_, sug)| {
                        let document_change = TextDocumentEdit {
                            text_document: OptionalVersionedTextDocumentIdentifier {
                                uri: uri.clone(),
                                version: None,
                            },
                            edits: vec![OneOf::Left(TextEdit {
                                range: Range { start, end },
                                new_text: found_word.clone(),
                            })],
                        };

                        CodeActionOrCommand::CodeAction(CodeAction {
                            title: format!("Change to \"{}\"", sug).into(),
                            edit: Some(WorkspaceEdit {
                                document_changes: Some(DocumentChanges::Edits(vec![
                                    document_change,
                                ])),
                                ..WorkspaceEdit::default()
                            }),

                            ..CodeAction::default()
                        })
                    })
                    .collect()
            } else {
                vec![]
            };

            actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                title: format!("Add \"{}\" to Dictionary", found_word).into(),
                command: Some(Command {
                    title: String::from("add"),
                    command: found_word.clone(),
                    ..Command::default()
                }),
                ..CodeAction::default()
            }));

            Ok(Some(actions))
        } else {
            Ok(Some(
                [CodeActionOrCommand::CodeAction(CodeAction {
                    title: "none found".into(),
                    kind: Some(CodeActionKind::REFACTOR),
                    edit: None,
                    ..CodeAction::default()
                })]
                .to_vec(),
            ))
        }
    }

    async fn code_action_resolve(&self, params: CodeAction) -> Result<CodeAction> {
        if let Some(command) = &params.command {
            if command.title == "add" {
                self.client
                    .show_message(
                        MessageType::LOG,
                        format!("add \"{}\" to dict", command.command),
                    )
                    .await;
            }
        }

        if let Some(edits) = &params.edit {
            if let Some(DocumentChanges::Edits(document_changes)) = &edits.document_changes {
                for document_change in document_changes.iter() {
                    for edit in document_change.edits.iter() {
                        if let OneOf::Left(TextEdit { range, new_text }) = edit {
                            self.state.write().unwrap().apply_change(
                                &document_change.text_document.uri,
                                &(TextDocumentContentChangeEvent {
                                    range: Some(range.clone()),
                                    text: new_text.to_string(),
                                    range_length: None,
                                }),
                            )
                        }
                    }
                }
            }
        }

        Ok(params)
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}
