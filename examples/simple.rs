use clap::Subcommand;
use repllet::{CliProcessor, CommandHandler, DefaultErrorHandler};

#[derive(Debug, Subcommand)]
pub enum SimpleCli {
    Test,
}

pub fn main() {
    let processor: CliProcessor<SimpleCli> =
        CliProcessor::new(MyCommandHandler {}, DefaultErrorHandler::default());
    processor.run().unwrap();
}

pub struct MyCommandHandler {}

impl CommandHandler<SimpleCli> for MyCommandHandler {
    fn handle_command(&self, command: SimpleCli) {
        println!("Yay! {:?}", command);
    }
}
