use std::convert::Infallible;

use clap::Parser;

use repellet::{ExecutionContext, ReplContext, ReplHandler, TermReader};
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
    let processor: ReplContext<SimpleCli, _> = ReplContext::new(reader, MyCommandHandler {});
    processor.run().unwrap();
}

pub struct MyCommandHandler {}

impl ReplHandler<SimpleCli> for MyCommandHandler {
    type Err = Infallible;
    fn on_command(&self, ctx: &mut ExecutionContext, command: SimpleCli) -> Result<(), Self::Err> {
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
