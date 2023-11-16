use backend::Backend;
use clap::Parser;
use std::path::PathBuf;
use tower_lsp::{LspService, Server};

mod backend;
mod source_file;
mod state;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// include spell checking
    #[arg(short, long)]
    spell: bool,

    /// dictionary files to parse for words
    #[arg(short, long, default_value = "/usr/share/dict/words")]
    dict: Vec<PathBuf>,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    println!("{:?}", args);

    let (service, socket) = LspService::new(|client| Backend::new(client));
    Server::new(stdin, stdout, socket).serve(service).await;
}
