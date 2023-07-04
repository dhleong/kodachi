use crate::app::processing::ansi::Ansi;
use crate::app::processing::text::{ProcessorOutputReceiver, TextProcessor};
use std::io::{self, stdout};
use std::io::{stderr, Write};
use std::sync::{Arc, Mutex};

use crate::cli::ui::{AnsiTerminalWriteUI, UiState};
use crate::daemon::channel::ChannelSource;
use crate::daemon::protocol::RequestIdGenerator;
use crate::daemon::responses::DaemonResponse;

struct TestBed<R: ProcessorOutputReceiver> {
    ui: R,
    processor: TextProcessor,
}

impl<R: ProcessorOutputReceiver> TestBed<R> {
    fn receive(&mut self, to_receive: &str) -> io::Result<()> {
        self.ui.begin_chunk()?;

        self.processor.process(to_receive.into(), &mut self.ui)?;

        self.ui.end_chunk()
    }
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
    let ui = AnsiTerminalWriteUI::create(state, 0, notifier, out);
    let processor = TextProcessor::default();
    let mut testbed = TestBed { ui, processor };

    testbed.receive("Output line 1\r\n")?;
    testbed.receive("Lorem ipsum dolor sit amit bacon")?;
    testbed.receive("~Lorem ipsum dolor sit amit bacon Lorem ipsum dolor sit amit bacon\r\n")?;
    testbed.receive("Output line 3\r\n")?;

    Ok(())
}
