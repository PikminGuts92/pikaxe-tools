use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct CreatorArgs {
    #[arg(help = "Path to input ark file", required = true)]
    pub ark_path: String,
    #[arg(short, long, help = "Default outfit to load (i.e. alterna1, grim)")]
    pub default_outfit: Option<String>,
}

impl CreatorArgs {
    pub fn init() -> Self {
        Self::parse()
    }
}