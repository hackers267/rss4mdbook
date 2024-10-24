use clap::Parser;
use std::ffi::OsString;

pub mod gen;
pub mod util;

#[derive(Debug, Parser)]
#[command(author, version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, clap::Parser)]
pub enum Commands {
    #[command(arg_required_else_help = false)]
    Gen {
        #[arg(value_name = "BOOK")]
        book: String,
        #[arg(short, long)]
        limit: Option<usize>,
        #[arg(short, long)]
        day: Option<usize>,
    },

    #[command(external_subcommand)]
    External(Vec<OsString>),
}

pub fn run() {
    let args = Cli::parse();
    match args.command {
        Commands::Gen { book, limit, day } => gen::exp(book, limit, day),
        Commands::External(args) => {
            println!("Calling out to {:?} with {:?}", &args[0], &args[1..]);
        }
    }
}
