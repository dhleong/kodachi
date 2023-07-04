pub mod prompts;

use crossterm::{
    cursor::{MoveToColumn, MoveToPreviousLine, RestorePosition, SavePosition},
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
        processing::{ansi::Ansi, text::ProcessorOutputReceiver},
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

    fn save_position(&mut self) -> io::Result<()> {
        let rendered_lines = self.internal.rendered_prompt_lines;
        if rendered_lines > 0 {
            // ::crossterm::queue!(self.output, MoveToPreviousLine(rendered_lines - 1))?;
            // self.internal.rendered_prompt_lines = 0;
        }
        // ::crossterm::queue!(self.output, SavePosition)
        Ok(())
    }

    fn restore_position(&mut self) -> io::Result<()> {
        // ::crossterm::queue!(self.output, RestorePosition)
        Ok(())
    }

    fn restore_printed_line(&mut self, columns: usize) -> io::Result<()> {
        let (width, _) = ::crossterm::terminal::size()?;
        let lines: u16 = columns.saturating_div(width as usize) as u16;
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

    fn clear_from_cursor_down(&mut self) -> io::Result<()> {
        self.internal.rendered_prompt_lines = 0;
        ::crossterm::queue!(self.output, Clear(ClearType::FromCursorDown))
    }

    fn reset_colors(&mut self) -> io::Result<()> {
        ::crossterm::queue!(self.output, ResetColor)
    }

    fn text(&mut self, text: Ansi) -> io::Result<()> {
        // Clear the prompts
        let state = self.state.lock().unwrap();
        let last_prompts_count = self.internal.rendered_prompt_lines;
        if last_prompts_count > 0 {
            self.internal.rendered_prompt_lines = 0;
            // let (_, h) = ::crossterm::terminal::size()?;
            // let (_, y) = ::crossterm::cursor::position()?;
            // ::crossterm::queue!(self.output, MoveTo(0, y - last_prompts_count))?;

            ::crossterm::queue!(self.output, MoveToPreviousLine(last_prompts_count - 1))?;
            ::crossterm::queue!(self.output, Clear(ClearType::FromCursorDown))?;

            // ::crossterm::queue!(self.output, MoveTo(x, y))?;
        }

        self.output.write_all(&text.as_bytes())?;

        let has_full_line = text.ends_with("\n");
        if !state.prompts.is_empty() && has_full_line {
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
