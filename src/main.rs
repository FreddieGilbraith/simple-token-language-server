use backend::Backend;
use clap::Parser;
use spell::Spell;
use std::{error::Error, path::PathBuf};
use tower_lsp::{LspService, Server};

mod backend;
mod source_file;
mod spell;
mod state;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// include spell checking
    #[arg(short, long)]
    spell: bool,

    /// system dictionary file to source words
    #[arg(short, long, default_value = "/usr/share/dict/words")]
    dict: PathBuf,

    /// user dictionary file to store user-defined words
    #[arg(short, long, default_value = "~/words")]
    user_dict: PathBuf,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let spell = if args.spell {
        Some(Spell::new(&args.dict).await?)
    } else {
        None
    };

    let (service, socket) = LspService::new(|client| Backend::new(client, spell));

    Server::new(stdin, stdout, socket).serve(service).await;

    Ok(())
}
