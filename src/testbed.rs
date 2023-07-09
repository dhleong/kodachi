use crossterm::cursor::{MoveToColumn, MoveToPreviousLine, RestorePosition, SavePosition};
use crossterm::terminal::{Clear, ClearType};

use crate::app::matchers::MatcherSpec;
use crate::app::processing::text::ProcessorOutputReceiver;
use crate::app::{Id, LockableState};
use crate::cli::ui::AnsiTerminalWriteUI;
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
        self.ui.begin_chunk()?;

        let processor = self
            .state
            .lock()
            .unwrap()
            .connections
            .get_processor(self.id);
        processor
            .unwrap()
            .lock()
            .unwrap()
            .process(to_receive.into(), &mut self.ui)?;

        self.ui.end_chunk()
    }

    pub fn register_prompt(&mut self, group_id: Id, prompt_index: usize, prompt: &str) {
        let matcher = MatcherSpec::Regex {
            options: Default::default(),
            source: prompt.to_string(),
        };
        register_prompt::try_handle(self.state.clone(), self.id, matcher, group_id, prompt_index);
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

    // // ::crossterm::execute!(stdout(), SavePosition)?;
    // stdout().write_all(b"Lorem Ipsum Dolor sit amet bacon brisket ")?;
    // ::crossterm::execute!(stdout(), MoveToColumn(1), Clear(ClearType::FromCursorDown))?;
    // stdout().write_all(b"bACON")?;
    // stdout().flush()?;
    // return Ok(());

    testbed.register_prompt(0, 0, "^Prompt ([a-c]+).*$");
    testbed.register_prompt(0, 1, "^Prompt ([d-f]+).*$");

    testbed.receive("Output line 1\r\nPrompt abc\r\nPrompt def")?;
    testbed.receive("\r\n")?;
    testbed.receive("Lorem ipsum dolor sit amit bacon")?;
    testbed.receive("~Lorem ipsum dolor sit amit bacon Lorem ipsum dolor sit amit bacon\r\n")?;

    // testbed.receive("Prompt cba\r\n")?;
    testbed.receive("Prompt cba")?;
    testbed.receive("\r\n")?;
    testbed.receive("Prompt fed")?;

    testbed.receive("\r\n")?;
    // println!("\n\n{}", testbed.ui.dump_state());

    testbed.receive("\r\nOutput line 2\r\n")?;
    testbed.receive("Lorem ipsum dolor sit amit bacon")?;
    testbed.receive("~Lorem ipsum dolor sit amit bacon Lorem ipsum dolor sit amit bacon\r\n")?;
    testbed.receive("Output line 3\r\n")?;

    testbed.receive("Prompt cab")?;
    testbed.receive("\r\n")?;
    testbed.receive("Prompt fde")?;

    // Send some text:
    // testbed.ui.print_local_send("(look)".to_string())?;
    testbed.receive("\r\n(look)\r\n")?;
    // testbed.receive("\r\n")?;
    // testbed.receive("(look)\r\n")?;

    testbed.receive("look1\r\n")?;
    testbed.receive("look2\r\n")?;

    testbed.receive("Prompt bca")?;
    testbed.receive("\r\n")?;
    testbed.receive("Prompt dfe")?;

    Ok(())
}
