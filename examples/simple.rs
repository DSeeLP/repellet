use std::error::Error;

use clap::Parser;

use repellet::{CliProcessor, CommandHandler, DefaultErrorHandler, ExecutionContext, TermReader};
use simplelog::{Config, TermLogger};

#[derive(Debug, Parser)]
pub enum SimpleCli {
    Test { name: String },
    Clear,
    Panic,
}

pub fn main() {
    let reader = TermReader::new();
    TermLogger::init(
        log::LevelFilter::Info,
        Config::default(),
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Auto,
    )
    .unwrap();
    let processor: CliProcessor<SimpleCli> =
        CliProcessor::new(reader, MyCommandHandler {}, DefaultErrorHandler::default());
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
            SimpleCli::Panic => panic!("Panic Test"),
        }
        Ok(())
    }
}
