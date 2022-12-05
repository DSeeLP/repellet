use std::any::Any;
use std::panic::catch_unwind;
use std::sync::Mutex;
use std::{error::Error, marker::PhantomData};

use std::fmt::{Debug, Display};

use clap::Command;

use clap::{error::ErrorKind, Error as ClapError};
use reedline::{DefaultPrompt, ExternalPrinter, Prompt, Reedline, Signal};

pub struct TermReader {
    pub editor: Reedline,
    pub prompt: Box<dyn Prompt + Send>,
    pub external_printer: ExternalPrinter<String>,
}

impl TermReader {
    pub fn new() -> TermReader {
        let external_printer = ExternalPrinter::default();
        let editor = Reedline::create().with_external_printer(external_printer.clone());
        let prompt = DefaultPrompt::new();
        Self {
            editor,
            prompt: Box::new(prompt),
            external_printer,
        }
    }
}

pub struct CliProcessor<C: clap::Parser> {
    command_handler: Box<dyn CommandHandler<C> + Send>,
    error_handler: Box<dyn ErrorHandler + Send>,
    pub command: Command,
    pub reader: TermReader,
    _data: PhantomData<C>,
}

impl<C: clap::Parser + Debug> CliProcessor<C> {
    pub fn new(
        reader: TermReader,
        command_handler: impl CommandHandler<C> + Send + 'static,
        error_handler: impl ErrorHandler + Send + 'static,
    ) -> Self {
        let mut command = C::command().multicall(true);
        command.build();

        Self {
            command_handler: Box::new(command_handler),
            error_handler: Box::new(error_handler),
            command,
            reader,
            _data: PhantomData,
        }
    }
}

pub struct ExecutionContext<'a> {
    pub editor: &'a mut Reedline,
    pub printer: &'a ExternalPrinter<String>,
    pub command: &'a mut Command,
}

impl<'a> ExecutionContext<'a> {
    #[inline]
    pub fn print(&self, display: impl Display) {
        self.printer.print(format!("{}", display)).unwrap();
    }

    #[inline]
    pub fn handle_error(&self, error: ClapError) {
        self.print(error.render());
    }

    pub fn error(&mut self, kind: ErrorKind, message: impl Display) -> ClapError {
        self.command.error(kind, message)
    }
}
pub trait CommandHandler<C: clap::Parser> {
    fn handle_command(&self, ctx: &mut ExecutionContext, command: C) -> Result<(), Box<dyn Error>>;
}

pub trait ErrorHandler {
    fn on_interrupt(&self) {
        std::process::exit(130);
    }

    fn on_eof(&self) {}
    fn on_clap_error(&self, error: ClapError) {
        match ClapError::kind(&error) {
            ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => {
                log::info!("{}", error);
            }
            _ => {
                log::info!("Invalid command {}", error);
            }
        }
    }

    fn on_panic(&self, err: Box<dyn Any + Send>) {
        log::error!("Command execution panicked!");
    }
}

#[derive(Debug, Default)]
pub struct DefaultErrorHandler {}

impl ErrorHandler for DefaultErrorHandler {}

impl<C: clap::Parser + Debug> CliProcessor<C> {
    pub fn run(mut self) -> Result<(), Box<dyn Error>> {
        let mut command = self.command.clone();
        loop {
            let sig = self.reader.editor.read_line(&*self.reader.prompt);
            match sig {
                Ok(Signal::Success(buffer)) => {
                    let mtx = Mutex::new(&mut self);
                    let cmd_mtx = Mutex::new(&mut command);
                    if let Err(err) = catch_unwind(|| {
                        let mut command = cmd_mtx.lock().unwrap();
                        mtx.lock().unwrap().execute_command(&mut *command, &buffer);
                        // self.execute_command(&mut command, &buffer);
                    }) {
                        self.error_handler.on_panic(err);
                    }
                }
                Ok(Signal::CtrlC | Signal::CtrlD) => {
                    self.error_handler.on_interrupt();
                }
                x => {
                    log::error!("Reed failed: {:?}", x);
                }
            }
        }
    }

    fn execute_command(&mut self, command: &mut Command, line: &str) {
        if line.is_empty() {
            return;
        }

        match command.try_get_matches_from_mut(line.split_whitespace()) {
            Ok(cli_raw) => {
                if let Ok(cli) = C::from_arg_matches(&cli_raw) {
                    let mut context = ExecutionContext {
                        editor: &mut self.reader.editor,
                        printer: &self.reader.external_printer,
                        command,
                    };
                    if let Err(err) = self.command_handler.handle_command(&mut context, cli) {
                        log::error!("An error occurred while executing a command! {}", err);
                    }
                }
            }
            Err(clap_err) => self.error_handler.on_clap_error(clap_err),
        }
    }
}
