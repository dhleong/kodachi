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
            text::{ProcessorOutputReceiver, ProcessorOutputReceiverFactory, SystemMessage},
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
}

impl<W: Write> ProcessorOutputReceiver for AnsiTerminalWriteUI<W> {
    fn end_chunk(&mut self) -> io::Result<()> {
        self.output.flush()
    }

    fn clear_partial_line(&mut self) -> io::Result<()> {
        let columns = self.internal.printed_columns;
        self.internal.printed_columns = 0;

        let (width, _) = ::crossterm::terminal::size()?;
        let printed_lines = if columns == 0 { 0 } else { columns / width + 1 };

        let extra_lines_to_clean = self.internal.rendered_prompt_lines;
        self.internal.rendered_prompt_lines = 0;

        let lines: u16 = printed_lines + extra_lines_to_clean;
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

    fn system(&mut self, message: SystemMessage) -> io::Result<()> {
        ::crossterm::queue!(self.output, ResetColor)?;
        self.clear_partial_line()?;
        self.new_line()?;
        self.text(match message {
            SystemMessage::ConnectionStatus(text) => text.into(),
        })?;
        self.finish_line()
    }

    fn new_line(&mut self) -> io::Result<()> {
        self.internal.printed_columns = 0;

        // Clear the prompts
        let last_prompts_count = self.internal.rendered_prompt_lines;
        if last_prompts_count > 0 {
            self.internal.rendered_prompt_lines = 0;

            ::crossterm::queue!(self.output, MoveToPreviousLine(last_prompts_count))?;
            ::crossterm::queue!(self.output, Clear(ClearType::FromCursorDown))?;
        }

        Ok(())
    }

    fn text(&mut self, mut text: Ansi) -> io::Result<()> {
        // TODO: compute *visible* columns
        self.internal.printed_columns += text.strip_ansi().len() as u16;

        self.output.write_all(&text.as_bytes())
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
