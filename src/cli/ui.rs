pub mod prompts;

use crossterm::{
    cursor::{MoveToColumn, MoveToPreviousLine},
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
            text::{ProcessorOutputReceiver, SystemMessage},
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

#[derive(Default)]
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
        if columns == 0 {
            return Ok(());
        }

        let (width, _) = ::crossterm::terminal::size()?;
        let printed_lines = columns / width;

        // Also clear any "clean" prompt lines when dirty, since we're preparing
        // to print a line, which will result in prompts being fully restored
        let state = self.state.lock().unwrap();
        let total_prompt_lines = state.prompts.len();
        let clean_prompt_lines = state.prompts.get_clean_lines();
        let extra_lines_to_clean = if clean_prompt_lines < total_prompt_lines {
            clean_prompt_lines as u16
        } else {
            0
        };

        let lines: u16 = printed_lines + extra_lines_to_clean;
        if lines == 0 {
            ::crossterm::queue!(
                self.output,
                MoveToColumn(1),
                Clear(ClearType::FromCursorDown)
            )
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
            SystemMessage::ConnectionStatus(text) | SystemMessage::LocalSend(text) => text.into(),
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

    #[cfg(dbg)]
    fn dump_state(&self) -> String {
        format!(
            "rendered={}; clean={}",
            self.internal.rendered_prompt_lines,
            self.state.lock().unwrap().prompts.get_clean_lines()
        )
    }

    fn text(&mut self, mut text: Ansi) -> io::Result<()> {
        // TODO: compute *visible* columns
        self.internal.printed_columns += text.strip_ansi().len() as u16;

        self.output.write_all(&text.as_bytes())
    }

    fn finish_line(&mut self) -> io::Result<()> {
        self.internal.printed_columns = 0;
        let state = self.state.lock().unwrap();
        if !state.prompts.is_empty() {
            let prompts_count = state.prompts.len() as u16;
            for prompt in state.prompts.iter() {
                if let Some(prompt) = prompt {
                    self.output.write_all(&prompt.as_bytes())?;

                    // NOTE: This can be convenient for testing redraws:
                    // self.output
                    //     .write_all(&format!("{:?}", SystemTime::now()).as_bytes())?;

                    self.output.write_all("\r\n".as_bytes())?;
                }
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
