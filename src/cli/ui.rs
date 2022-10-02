use crossterm::{
    cursor::{RestorePosition, SavePosition},
    terminal::{Clear, ClearType},
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
}

impl<W: Write> ProcessorOutputReceiver for AnsiTerminalWriteUI<W> {
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
        self.output.write_all(&text.as_bytes())
    }

    fn notification(&mut self, notification: DaemonNotification) -> io::Result<()> {
        self.notifier.notify(Notification::ForConnection {
            connection_id: self.connection_id,
            notification,
        });
        Ok(())
    }
}
