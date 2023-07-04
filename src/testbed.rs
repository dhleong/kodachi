use crate::app::processing::ansi::Ansi;
use crate::app::processing::text::ProcessorOutputReceiver;
use std::io::{self, stdout};
use std::io::{stderr, Write};
use std::sync::{Arc, Mutex};

use crate::cli::ui::{AnsiTerminalWriteUI, UiState};
use crate::daemon::channel::ChannelSource;
use crate::daemon::protocol::RequestIdGenerator;
use crate::daemon::responses::DaemonResponse;

fn receive<T: ProcessorOutputReceiver>(ui: &mut T, to_receive: &str) -> io::Result<()> {
    ui.begin_chunk()?;
    ui.text(to_receive.into())?;
    ui.end_chunk()
}

pub fn run() -> io::Result<()> {
    let out = stdout();
    let state: Arc<Mutex<UiState>> = Default::default();
    {
        let mut s = state.lock().unwrap();
        s.prompts.set_index(0, Ansi::from("Prompt 0\r\n"));
        s.prompts.set_index(1, Ansi::from("Prompt 1"));
    }

    let write: Box<dyn Write + Send> = Box::new(stderr());
    let (sender, _) = tokio::sync::broadcast::channel(1);
    let channels = ChannelSource::new(write, RequestIdGenerator::default(), sender);
    let notifier = channels
        .create_with_request_id(0)
        .respond(DaemonResponse::OkResult);
    let mut ui = AnsiTerminalWriteUI::create(state, 0, notifier, out);

    receive(&mut ui, "Output line 1\r\n")?;
    receive(
        &mut ui,
        "Lorem ipsum dolor sit amit bacon Lorem ipsum dolor sit amit bacon Lorem ipsum dolor sit amit bacon\r\n",
    )?;
    receive(&mut ui, "Output line 3\r\n")?;

    Ok(())
}
