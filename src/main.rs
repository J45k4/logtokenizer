use args::Args;
use args::Commands;
use clap::Parser;
use drain_cmd::drain_cmd;

mod drain;
mod args;
mod drain_cmd;
mod utility;

fn main() {
    let args: Args = Args::parse();

    match args.command {
        Commands::Drain(drain_args) => {
            drain_cmd(drain_args);
        }
    }
}
