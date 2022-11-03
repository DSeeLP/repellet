use std::error::Error;

use clap::Subcommand;
use repllet::{CliProcessor, CommandHandler, DefaultErrorHandler};

#[derive(Debug, Subcommand)]
pub enum SimpleCli {
    Test { name: String },
    Lol,
}

pub fn main() {
    let mut processor: CliProcessor<SimpleCli> =
        CliProcessor::new(MyCommandHandler {}, DefaultErrorHandler::default());
    processor.run().unwrap();
}

pub struct MyCommandHandler {}

impl CommandHandler<SimpleCli> for MyCommandHandler {
    fn handle_command(&self, command: SimpleCli) -> Result<(), Box<dyn Error>> {
        match command {
            SimpleCli::Test { name } => {
                println!("Test from name {}", name)
            }
            SimpleCli::Lol => {
                println!("Lol")
            }
        }
        Ok(())
    }
}
