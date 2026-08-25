use crate::app::{persistence::PersistedLines, processing::text::ProcessorOutputReceiver};

pub struct PersistentOutputHandlingReceiver<T: ProcessorOutputReceiver> {
    pub base: T,
    pub persisted_output: Option<PersistedLines>,
}

impl<T: ProcessorOutputReceiver> ProcessorOutputReceiver for PersistentOutputHandlingReceiver<T> {
    fn window_size_source(&self) -> Option<crate::app::processing::text::WindowSizeSource> {
        self.base.window_size_source()
    }

    fn new_line(&mut self) -> std::io::Result<()> {
        if let Some(output) = self.persisted_output.as_ref() {
            output.push_empty_line();
        }
        self.base.new_line()
    }

    fn finish_line(&mut self) -> std::io::Result<()> {
        if let Some(output) = self.persisted_output.as_ref() {
            output.flush()?;
        }
        self.base.finish_line()
    }

    fn clear_partial_line(&mut self) -> std::io::Result<()> {
        if let Some(output) = self.persisted_output.as_ref() {
            output.clear_last_line();
        }
        self.base.clear_partial_line()
    }

    fn text(&mut self, text: crate::app::processing::ansi::Ansi) -> std::io::Result<()> {
        if let Some(output) = self.persisted_output.as_ref() {
            output.append_to_last_line(&text);
        }
        self.base.text(text)
    }

    fn system(&mut self, text: crate::app::processing::text::SystemMessage) -> std::io::Result<()> {
        // TODO: Add to persisted_output?
        self.base.system(text)
    }

    fn notification(
        &mut self,
        notification: crate::daemon::notifications::DaemonNotification,
    ) -> std::io::Result<()> {
        self.base.notification(notification)
    }
}
