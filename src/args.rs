use clap::Parser;
use clap::Subcommand;

#[derive(Debug, Parser)]
#[clap(name = "drain")]
pub struct Args {
    #[clap(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    #[clap(arg_required_else_help = true)]
    Drain(DrainArgs),
}

#[derive(Debug, Parser)]
pub struct DrainArgs {
    #[arg(short, long)]
    pub input_path: String,
    #[arg(short, long)]
    pub save_tokens: Option<String>,
    #[arg(short, long)]
    pub save_templates: Option<String>,
    #[arg(short, long)]
    pub save_csv: Option<String>,
}