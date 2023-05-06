pub mod prompts;

use crossterm::{
    cursor::{MoveTo, MoveToNextLine, RestorePosition, SavePosition},
    style::ResetColor,
    terminal::{Clear, ClearType, ScrollDown, ScrollUp},
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
        ::crossterm::queue!(self.output, SavePosition)
    }

    fn restore_position(&mut self) -> io::Result<()> {
        ::crossterm::queue!(self.output, RestorePosition)
    }

    fn clear_from_cursor_down(&mut self) -> io::Result<()> {
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
            ::crossterm::queue!(self.output, ScrollDown(last_prompts_count))?;
        }

        self.output.write_all(&text.as_bytes())?;

        if !state.prompts.is_empty() {
            let (_, h) = ::crossterm::terminal::size()?;
            let (x, y) = ::crossterm::cursor::position()?;

            let prompts_count = state.prompts.len() as u16;
            ::crossterm::queue!(self.output, ScrollUp(prompts_count))?;
            ::crossterm::queue!(self.output, MoveTo(0, h - prompts_count))?;

            for prompt in state.prompts.iter() {
                if let Some(prompt) = prompt {
                    self.output.write_all(&prompt.as_bytes())?;

                    // NOTE: This can be convenient for testing redraws:
                    // self.output
                    //     .write_all(&format!("{:?}", SystemTime::now()).as_bytes())?;

                    ::crossterm::queue!(self.output, MoveToNextLine(1))?;
                }
            }

            ::crossterm::queue!(self.output, MoveTo(x, y + prompts_count),)?;
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
