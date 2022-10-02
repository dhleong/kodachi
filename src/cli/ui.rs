use crossterm::{
    cursor::{MoveTo, MoveToNextLine, RestorePosition, SavePosition},
    terminal::{Clear, ClearType, ScrollDown, ScrollUp},
};
use std::io::{self, Write};

use crate::{
    app::{
        processing::{ansi::Ansi, text::ProcessorOutputReceiver},
        Id,
    },
    daemon::{
        channel::RespondedChannel, notifications::DaemonNotification, protocol::Notification,
    },
};

/// This UI expects to interact with an ANSI-powered terminal UI
/// via an object that implements Write
pub struct AnsiTerminalWriteUI<W: Write> {
    pub connection_id: Id,
    pub notifier: RespondedChannel,
    pub output: W,

    prompts: Vec<Ansi>,
}

impl<W: Write> AnsiTerminalWriteUI<W> {
    pub fn create(connection_id: Id, notifier: RespondedChannel, output: W) -> Self {
        Self {
            connection_id,
            notifier,
            output,
            prompts: vec![],
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

    fn text(&mut self, text: Ansi) -> io::Result<()> {
        // Clear the prompts
        let prompts_count = self.prompts.len() as u16;
        ::crossterm::queue!(self.output, ScrollDown(prompts_count))?;

        self.output.write_all(&text.as_bytes())?;

        let (_, h) = ::crossterm::terminal::size()?;
        let (x, y) = ::crossterm::cursor::position()?;

        if !self.prompts.is_empty() {
            ::crossterm::queue!(self.output, ScrollUp(prompts_count))?;
            ::crossterm::queue!(self.output, MoveTo(0, h - prompts_count))?;

            for prompt in &self.prompts {
                self.output.write_all(&prompt.as_bytes())?;

                // NOTE: This can be convenient for testing redraws:
                // self.output
                //     .write_all(&format!("{:?}", SystemTime::now()).as_bytes())?;

                ::crossterm::queue!(self.output, MoveToNextLine(1))?;
            }

            ::crossterm::queue!(self.output, MoveTo(x, y),)?;
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
