use crate::app::matchers::MatcherSpec;
use crate::app::processing::text::ProcessorOutputReceiver;
use crate::app::{Id, LockableState};
use crate::cli::ui::AnsiTerminalWriteUI;
use crate::daemon::handlers::connect::{handle_received_text, handle_sent_text};
use crate::daemon::handlers::register_prompt;
use std::io::{self, stdout};
use std::io::{stderr, Write};

use crate::daemon::channel::ChannelSource;
use crate::daemon::protocol::RequestIdGenerator;
use crate::daemon::responses::DaemonResponse;

struct TestBed<R: ProcessorOutputReceiver> {
    state: LockableState,
    ui: R,
    id: Id,
}

impl<R: ProcessorOutputReceiver> TestBed<R> {
    fn receive(&mut self, to_receive: &str) -> io::Result<()> {
        let processor = &self
            .state
            .lock()
            .unwrap()
            .connections
            .get_processor(self.id)
            .unwrap();
        handle_received_text(&mut self.ui, processor, to_receive.into())
    }

    fn send(&mut self, to_send: &str) -> io::Result<()> {
        let processor = &self
            .state
            .lock()
            .unwrap()
            .connections
            .get_processor(self.id)
            .unwrap();
        handle_sent_text(&mut self.ui, processor, to_send.to_string())
    }

    pub fn register_prompt(&mut self, group_id: Id, prompt_index: usize, prompt: &str) {
        let matcher = MatcherSpec::Regex {
            options: Default::default(),
            source: prompt.to_string(),
        };

        register_prompt::try_handle(
            None,
            self.state.clone(),
            self.id,
            matcher,
            group_id,
            prompt_index,
        );
    }
}

pub fn run() -> io::Result<()> {
    let out = stdout();

    let write: Box<dyn Write + Send> = Box::new(stderr());
    let (sender, _) = tokio::sync::broadcast::channel(1);
    let channels = ChannelSource::new(write, RequestIdGenerator::default(), sender);
    let notifier = channels
        .create_with_request_id(0)
        .respond(DaemonResponse::OkResult);

    let mut state = LockableState::default();
    let connection = state.lock().unwrap().connections.create();
    let ui = AnsiTerminalWriteUI::create(connection.state.ui_state.clone(), 0, notifier, out);
    let mut testbed = TestBed {
        state,
        id: connection.id,
        ui,
    };

    testbed.register_prompt(0, 0, "^Prompt ([a-c]+).*$");
    testbed.register_prompt(0, 1, "^Prompt ([d-f]+).*$");

    testbed.receive("Output line 1\r\nPrompt abc\r\nPrompt def")?;
    testbed.receive("\r\n")?;
    testbed.receive("Lorem ipsum dolor sit amit bacon")?;
    testbed.receive("~Lorem ipsum dolor sit amit bacon Lorem ipsum dolor sit amit bacon\r\n")?;

    testbed.receive("Prompt cba")?;
    testbed.receive("\r\n")?;
    testbed.receive("Prompt fed")?;

    testbed.receive("\r\n")?;

    testbed.receive("\r\nOutput line 2\r\n")?;
    testbed.receive("Lorem ipsum dolor sit amit bacon")?;
    testbed.receive("~Lorem ipsum dolor sit amit bacon Lorem ipsum dolor sit amit bacon\r\n")?;
    testbed.receive("Output line 3\r\n")?;

    testbed.receive("Prompt cab")?;
    testbed.receive("\r\n")?;
    testbed.receive("Prompt fde")?;

    // Send some text:
    testbed.send("(look)")?;

    testbed.receive("look1\r\n")?;
    testbed.receive("look2\r\n")?;

    testbed.receive("Prompt bca")?;
    testbed.receive("\r\n")?;
    testbed.receive("Prompt dfe")?;

    Ok(())
}
