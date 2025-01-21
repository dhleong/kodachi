use std::{collections::HashSet, env};

pub struct KodachiLogger {
    pub enabled_namespaces: HashSet<String>,
    pub allow_any: bool,
}

impl Default for KodachiLogger {
    fn default() -> Self {
        let string = env::var("DEBUG").unwrap_or_default();
        let enabled_namespaces = string.split(',').map(|s| s.to_string()).collect();
        KodachiLogger {
            allow_any: string == "*" || string == "1",
            enabled_namespaces,
        }
    }
}

impl log::Log for KodachiLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        self.allow_any || self.enabled_namespaces.contains(metadata.target())
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            println!("[{}] {}", record.target(), record.args());
        }
    }

    fn flush(&self) {
        // nop
    }
}
