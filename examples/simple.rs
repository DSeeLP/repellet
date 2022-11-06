use std::error::Error;

use clap::Parser;

use repllet::{CliProcessor, CommandHandler, DefaultErrorHandler, ExecutionContext};

#[derive(Debug, Parser)]
pub enum SimpleCli {
    Test { name: String },
    Clear,
}

pub fn main() {
    let processor: CliProcessor<SimpleCli> =
        CliProcessor::new(MyCommandHandler {}, DefaultErrorHandler::default());
    processor.run().unwrap();
}

pub struct MyCommandHandler {}

impl CommandHandler<SimpleCli> for MyCommandHandler {
    fn handle_command(
        &self,
        ctx: &mut ExecutionContext,
        command: SimpleCli,
    ) -> Result<(), Box<dyn Error>> {
        match command {
            SimpleCli::Test { name } => {
                println!("Test from name {}", name)
            }
            SimpleCli::Clear => {
                ctx.editor.clear_scrollback().unwrap();
            }
        }
        Ok(())
    }
}
