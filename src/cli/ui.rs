pub mod external;
pub mod prompts;

use crossterm::{
    cursor::MoveToPreviousLine,
    style::ResetColor,
    terminal::{Clear, ClearType},
};
use std::{
    io::{self, Write},
    sync::{Arc, Mutex},
};

use crate::{
    app::{
        clearable::Clearable,
        processing::{
            ansi::Ansi,
            text::{
                ProcessorOutputReceiver, ProcessorOutputReceiverFactory, SystemMessage,
                WindowSizeSource,
            },
        },
        Id,
    },
    daemon::{
        channel::RespondedChannel, notifications::DaemonNotification, protocol::Notification,
    },
};

use self::prompts::{PromptGroups, PromptsState};

#[derive(Default)]
pub struct UiState {
    pub prompts: PromptsState,
    pub active_prompt_group: Id,
    pub inactive_prompt_groups: PromptGroups,
}

impl Clearable for UiState {
    fn clear(&mut self) {
        self.prompts.clear();
        self.inactive_prompt_groups.clear();
    }
}

#[derive(Debug, Default)]
struct InternalState {
    rendered_prompt_lines: u16,
    printed_columns: u16,
}

/// This UI expects to interact with an ANSI-powered terminal UI
/// via an object that implements Write
pub struct AnsiTerminalWriteUI<W: Write> {
    pub connection_id: Id,
    pub notifier: RespondedChannel,
    pub output: W,

    state: Arc<Mutex<UiState>>,
    internal: InternalState,
}

impl<W: Write> AnsiTerminalWriteUI<W> {
    pub fn create(
        state: Arc<Mutex<UiState>>,
        connection_id: Id,
        notifier: RespondedChannel,
        output: W,
    ) -> Self {
        Self {
            connection_id,
            notifier,
            output,
            state,
            internal: InternalState::default(),
        }
    }

    fn clear_prompts_plus_lines(&mut self, extra_lines: u16) -> io::Result<()> {
        let prompt_lines = self.internal.rendered_prompt_lines;
        self.internal.rendered_prompt_lines = 0;

        let lines: u16 = extra_lines + prompt_lines;
        if lines == 0 {
            // nop? Already on a cleared line
            Ok(())
        } else {
            ::crossterm::queue!(
                self.output,
                MoveToPreviousLine(lines),
                Clear(ClearType::FromCursorDown)
            )
        }
    }
}

impl<W: Write> ProcessorOutputReceiver for AnsiTerminalWriteUI<W> {
    fn window_size_source(&self) -> Option<WindowSizeSource> {
        Some(WindowSizeSource::Crossterm)
    }

    fn end_chunk(&mut self) -> io::Result<()> {
        self.output.flush()
    }

    fn clear_partial_line(&mut self) -> io::Result<()> {
        let columns = self.internal.printed_columns;
        self.internal.printed_columns = 0;

        let (width, _) = ::crossterm::terminal::size()?;
        let printed_lines = if columns == 0 { 0 } else { columns / width + 1 };

        self.clear_prompts_plus_lines(printed_lines)
    }

    fn system(&mut self, message: SystemMessage) -> io::Result<()> {
        self.clear_prompts_plus_lines(0)?;
        ::crossterm::queue!(self.output, ResetColor)?;
        self.text(match message {
            SystemMessage::ConnectionStatus(text) => text.into(),
            SystemMessage::LocalSend(text) => text.into(),
        })?;
        self.new_line()?;
        self.finish_line()
    }

    fn new_line(&mut self) -> io::Result<()> {
        // Since we're starting a new line, we need to reset internal state
        self.internal.printed_columns = 0;
        self.output.write_all("\r\n".as_bytes())?;
        Ok(())
    }

    fn text(&mut self, text: Ansi) -> io::Result<()> {
        // TODO: compute *visible* columns
        // NOTE: In order to ensure consistent newline handling,
        // we strip off whatever (possibly bogus) line endings
        // we were given, and print our own in new_line
        let mut trimmed = text.trim_trailing_newlines();
        self.internal.printed_columns += trimmed.strip_ansi().len() as u16;

        self.output.write_all(&trimmed.as_bytes())
    }

    fn finish_line(&mut self) -> io::Result<()> {
        let state = self.state.lock().unwrap();
        if !state.prompts.is_empty() {
            // If we have a partial line, go to a new line to print our prompts
            if self.internal.printed_columns > 0 {
                self.output.write_all("\r\n".as_bytes())?;
            }

            let prompts_count = state.prompts.len() as u16;
            for prompt in state.prompts.iter().flatten() {
                self.output.write_all(&prompt.as_bytes())?;

                // NOTE: This can be convenient for testing redraws:
                // self.output
                //     .write_all(&format!("{:?}", SystemTime::now()).as_bytes())?;

                self.output.write_all("\r\n".as_bytes())?;
            }

            self.internal.rendered_prompt_lines = prompts_count;
        }
        Ok(())
    }

    fn notification(&mut self, notification: DaemonNotification) -> io::Result<()> {
        self.notifier.notify(Notification::ForConnection {
            connection_id: self.connection_id,
            notification,
        });
        Ok(())
    }
}

#[derive(Clone, Copy)]
pub struct StdoutAnsiTerminalWriteUIFactory;

impl ProcessorOutputReceiverFactory for StdoutAnsiTerminalWriteUIFactory {
    type Implementation = AnsiTerminalWriteUI<io::Stdout>;

    fn create(
        &self,
        state: Arc<Mutex<UiState>>,
        connection_id: Id,
        notifier: RespondedChannel,
    ) -> Self::Implementation {
        AnsiTerminalWriteUI::create(state, connection_id, notifier, io::stdout())
    }
}
