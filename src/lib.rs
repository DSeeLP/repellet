use std::any::Any;
use std::marker::PhantomData;
use std::panic::catch_unwind;
use std::sync::Mutex;

use std::fmt::{Debug, Display};

use clap::Command;

use clap::error::RichFormatter;
use clap::{error::ErrorKind, Error as ClapError};
use reedline::{DefaultPrompt, DefaultPromptSegment, ExternalPrinter, Prompt, Reedline, Signal};
use thiserror::Error;

#[cfg(feature = "default_error_handler")]
#[cfg(not(any(feature = "tracing", feature = "log")))]
compile_error!("Feature 'tracing' or 'log' must be activated");

#[cfg(feature = "static_prompt")]
mod prompt;
#[cfg(feature = "static_prompt")]
pub use prompt::*;

pub struct TermReader {
    pub editor: Reedline,
    pub prompt: Box<dyn Prompt + Send>,
    pub external_printer: ExternalPrinter<String>,
}

impl TermReader {
    pub fn new() -> TermReader {
        let external_printer = ExternalPrinter::default();
        let editor = Reedline::create().with_external_printer(external_printer.clone());
        let prompt = DefaultPrompt::new(
            DefaultPromptSegment::Basic("> ".into()),
            DefaultPromptSegment::Empty,
        );
        Self {
            editor,
            prompt: Box::new(prompt),
            external_printer,
        }
    }

    pub fn set_prompt<P: Prompt + Send + 'static>(&mut self, prompt: P) {
        self.prompt = Box::new(prompt);
    }
}

pub struct ReplContext<C: clap::Parser, Err: Debug + Display> {
    handler: Box<dyn ReplHandler<C, Err = Err> + Send>,
    pub command: Command,
    pub reader: TermReader,
    _data: PhantomData<C>,
}

impl<C: clap::Parser + Debug, Err: Debug + Display> ReplContext<C, Err> {
    pub fn new(
        reader: TermReader,
        handler: impl ReplHandler<C, Err = Err> + Send + 'static,
    ) -> Self {
        let mut command = C::command().multicall(true);
        command.build();

        Self {
            handler: Box::new(handler),
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

pub trait ReplHandler<C: clap::Parser> {
    type Err: Debug + Display;
    fn on_command(&self, ctx: &mut ExecutionContext, command: C) -> Result<(), Self::Err>;
}

#[derive(Debug, Error)]
pub enum ReplError<Err: Debug + Display> {
    #[error("Read was interrupted")]
    Interrupt,
    #[error("EOF occurred")]
    EOF,
    #[error(transparent)]
    Clap(#[from] ClapError),
    #[error("{0}")]
    Parse(clap::error::Error<RichFormatter>),
    #[error("Command execution panicked")]
    Panic(Box<dyn Any + Send>),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("An error occurred while executing a command {0}")]
    ExecutionError(Err),
}

pub trait ErrorHandler<Err: Debug + Display>:
    Fn(ReplError<Err>) -> Result<(), ReplError<Err>>
{
}

impl<Err: ReplExecutionError, F: Fn(ReplError<Err>) -> Result<(), ReplError<Err>>> ErrorHandler<Err>
    for F
{
}

pub trait ReplExecutionError: Debug + Display {}

impl<T: Debug + Display> ReplExecutionError for T {}

#[cfg(feature = "default_error_handler")]
#[allow(unused_variables)]
pub fn default_error_handler<Err: ReplExecutionError>(
    error: ReplError<Err>,
) -> Result<(), ReplError<Err>> {
    match &error {
        ReplError::Interrupt | ReplError::EOF => Err(error),
        ReplError::Clap(err) => {
            match err.kind() {
                ErrorKind::DisplayHelp
                | ErrorKind::DisplayVersion
                | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand => {
                    let err = err.render();
                    #[cfg(feature = "tracing")]
                    tracing::warn!("{}", err);
                    #[cfg(feature = "log")]
                    log::warn!("{}", err);
                }
                err => {
                    #[cfg(feature = "tracing")]
                    tracing::warn!("{}", err);
                    #[cfg(feature = "log")]
                    log::warn!("{}", err);
                }
            };
            Ok(())
        }
        ReplError::Parse(err) => {
            #[cfg(feature = "tracing")]
            tracing::warn!("{}", err);
            #[cfg(feature = "log")]
            log::warn!("{}", err);
            Ok(())
        }
        ReplError::Panic(_) => Err(error),
        ReplError::Io(_) => Err(error),
        ReplError::ExecutionError(err) => {
            #[cfg(feature = "tracing")]
            tracing::warn!("{}", err);
            #[cfg(feature = "log")]
            log::warn!("{}", err);
            Ok(())
        }
    }
}

impl<C: clap::Parser + Debug, Err: Debug + Display> ReplContext<C, Err> {
    pub fn read_loop<F: ErrorHandler<Err>>(
        mut self,
        handle_error: F,
    ) -> Result<(), ReplError<Err>> {
        let command = &mut self.command.clone();
        loop {
            match self.read_with_command(command) {
                Ok(_) => {}
                Err(err) => {
                    if let Err(err) = handle_error(err) {
                        return Err(err);
                    }
                }
            }
        }
    }
    pub fn read(&mut self) -> Result<(), ReplError<Err>> {
        self.read_with_command(&mut self.command.clone())
    }

    pub fn read_with_command(&mut self, command: &mut Command) -> Result<(), ReplError<Err>> {
        let sig = self.reader.editor.read_line(&*self.reader.prompt);
        match sig {
            Ok(Signal::Success(buffer)) => {
                let mtx = Mutex::new(self);
                let cmd_mtx = Mutex::new(command);
                let catch_res = catch_unwind(|| {
                    let mut command = cmd_mtx.lock().unwrap();
                    mtx.lock().unwrap().execute_command(&mut *command, &buffer)
                    // self.execute_command(&mut command, &buffer);
                });
                match catch_res {
                    Ok(res) => res,
                    Err(err) => Err(ReplError::Panic(err)),
                }
            }
            Ok(Signal::CtrlC) => Err(ReplError::Interrupt),
            Ok(Signal::CtrlD) => Err(ReplError::EOF),
            Err(err) => Err(ReplError::Io(err)),
        }
    }

    fn execute_command(&mut self, command: &mut Command, line: &str) -> Result<(), ReplError<Err>> {
        if line.is_empty() {
            return Ok(());
        }

        match command.try_get_matches_from_mut(line.split_whitespace()) {
            Ok(cli_raw) => match C::from_arg_matches(&cli_raw) {
                Ok(cli) => {
                    let mut context = ExecutionContext {
                        editor: &mut self.reader.editor,
                        printer: &self.reader.external_printer,
                        command,
                    };
                    self.handler
                        .on_command(&mut context, cli)
                        .map_err(|err| ReplError::ExecutionError(err))
                }
                Err(err) => Err(ReplError::Parse(err)),
            },
            Err(err) => Err(ReplError::Parse(err)),
        }
    }
}
