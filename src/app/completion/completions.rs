use std::collections::HashSet;

use regex::Regex;

use crate::app::processing::ansi::Ansi;

use super::CompletionParams;

pub struct Completions {
    incoming_words: HashSet<String>,
}

impl Default for Completions {
    fn default() -> Self {
        Completions {
            incoming_words: Default::default(),
        }
    }
}

impl Completions {
    pub fn process_incoming(&mut self, line: &mut Ansi) {
        let words_regex = Regex::new(r"(\w+)").unwrap();
        for m in words_regex.find_iter(&line.strip_ansi()) {
            let word = m.as_str();
            if !self.incoming_words.contains(word) {
                self.incoming_words.insert(word.to_string());
            }
        }
    }

    pub fn suggest(&self, _params: CompletionParams) -> Vec<String> {
        // TODO
        self.incoming_words.iter().map(|s| s.to_string()).collect()
    }
}
