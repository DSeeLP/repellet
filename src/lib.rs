use std::collections::HashMap;
use std::sync::Mutex;
use std::{error::Error, marker::PhantomData};

use std::fmt::{Debug, Display};

use clap::Command;

use clap::error::{ContextKind, ContextValue};
use clap::{error::ErrorKind, Error as ClapError};
use nu_ansi_term::{Color, Style};
use reedline::{DefaultPrompt, ExternalPrinter, Highlighter, Prompt, Reedline, Signal, StyledText};

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

    pub fn with_highlighter(mut self) -> Self {
        self.editor = self.editor.with_highlighter(Box::new(SimpleHighlighter {
            command: Mutex::new(self.command.clone()),
        }));
        self
    }
}

pub struct SimpleHighlighter {
    command: Mutex<Command>,
}

impl Highlighter for SimpleHighlighter {
    fn highlight(&self, line: &str, cursor: usize) -> reedline::StyledText {
        let mut styled = StyledText::new();
        let lock = &mut *self.command.lock().unwrap();
        let res = lock.try_get_matches_from_mut(line.split_whitespace());
        let ok_style = Style::new().fg(Color::Green);
        let err_style = Style::new().fg(Color::LightRed);
        styled.push((Style::new(), line.into()));
        match res {
            Ok(ok) => {
                println!("Ok: {:?}", ok);
            }
            Err(err) => {
                let mut context: HashMap<ContextKind, ContextValue> =
                    err.context().map(|v| (v.0, v.1.clone())).collect();
                context.remove(&ContextKind::Usage);
                context.remove(&ContextKind::Suggested);
                match err.kind() {
                    // ErrorKind::InvalidValue => todo!(),
                    // ErrorKind::UnknownArgument => todo!(),
                    // ErrorKind::InvalidSubcommand => todo!(),
                    // ErrorKind::ArgumentConflict => todo!(),
                    // ErrorKind::MissingRequiredArgument => todo!(),
                    // ErrorKind::MissingSubcommand => todo!(),
                    // ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand => todo!(),
                    kind => {
                        println!("Unhandled kind: {:?}", kind);
                    }
                }
                println!("Kind: '{:?}', Context: {:?}", err.kind(), context);
            }
        }
        styled
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
                println!("{}", error);
            }
            _ => {
                eprintln!("Invalid command {}", error);
            }
        }
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
            Ok(cli_raw) => {
                if let Ok(cli) = C::from_arg_matches(&cli_raw) {
                    let mut context = ExecutionContext {
                        editor: &mut self.editor,
                        printer: &self.printer,
                        command,
                    };
                    if let Err(err) = self.command_handler.handle_command(&mut context, cli) {
                        eprintln!("An error occurred while executing a command! {}", err);
                    }
                }
            }
            Err(clap_err) => self.error_handler.on_clap_error(clap_err),
        }
    }
}
