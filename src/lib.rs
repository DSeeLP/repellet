use std::{error::Error, marker::PhantomData};

use std::fmt::Debug;

use clap::Command;

use clap::{error::ErrorKind, Error as ClapError};
use reedline::{DefaultPrompt, ExternalPrinter, Prompt, Reedline, Signal};

pub struct CliProcessor<C: clap::Parser> {
    command_handler: Box<dyn CommandHandler<C> + Send>,
    error_handler: Box<dyn ErrorHandler + Send>,
    pub command: Command,
    pub editor: Reedline,
    pub prompt: Box<dyn Prompt + Send>,
    pub printer: ExternalPrinter<String>,
    _data: PhantomData<C>,
}

impl<C: clap::Parser + Debug> CliProcessor<C> {
    pub fn new(
        command_handler: impl CommandHandler<C> + Send + 'static,
        error_handler: impl ErrorHandler + Send + 'static,
    ) -> Self {
        let mut command = C::command().multicall(true);
        command.build();
        let printer = ExternalPrinter::default();
        let editor = Reedline::create().with_external_printer(printer.clone());
        let prompt = DefaultPrompt::new();

        Self {
            command_handler: Box::new(command_handler),
            error_handler: Box::new(error_handler),
            command,
            editor,
            prompt: Box::new(prompt),
            printer,
            _data: PhantomData,
        }
    }
}

pub trait CommandHandler<C: clap::Parser> {
    fn handle_command(&self, editor: &mut Reedline, command: C) -> Result<(), Box<dyn Error>>;
}

pub trait ErrorHandler {
    fn on_interrupt(&self) {
        std::process::exit(130);
    }

    fn on_eof(&self) {}
    fn on_clap_error(&self, error: ClapError) {
        match ClapError::kind(&error) {
            ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => {
                println!("{}", error);
            }
            _ => {
                eprintln!("Invalid command {}", error);
            }
        }
    }
}

impl dyn ErrorHandler {
    pub fn default() -> DefaultErrorHandler {
        DefaultErrorHandler {}
    }
}

#[derive(Debug, Default)]
pub struct DefaultErrorHandler {}

impl ErrorHandler for DefaultErrorHandler {}

impl<C: clap::Parser + Debug> CliProcessor<C> {
    pub fn run(mut self) -> Result<(), Box<dyn Error>> {
        let mut command = self.command.clone();
        loop {
            let sig = self.editor.read_line(&*self.prompt);
            match sig {
                Ok(Signal::Success(buffer)) => {
                    self.execute_command(&mut command, &buffer);
                }
                Ok(Signal::CtrlC | Signal::CtrlD) => {
                    self.error_handler.on_interrupt();
                }
                x => {
                    println!("Signal: {:?}", x);
                }
            }
        }
    }

    fn execute_command(&mut self, command: &mut Command, line: &str) {
        if line.is_empty() {
            return;
        }

        match command.try_get_matches_from_mut(line.split_whitespace()) {
            Ok(cli) => {
                if let Ok(cli) = C::from_arg_matches(&cli) {
                    if let Err(err) = self.command_handler.handle_command(&mut self.editor, cli) {
                        eprintln!("An error occurred while executing a command! {}", err);
                    }
                }
            }
            Err(clap_err) => self.error_handler.on_clap_error(clap_err),
        }
    }
}
