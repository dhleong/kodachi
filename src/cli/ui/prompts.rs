use crate::app::{clearable::Clearable, processing::ansi::Ansi};

#[derive(Default)]
pub struct PromptsState {
    values: Vec<Option<Ansi>>,
}

impl Clearable for PromptsState {
    fn clear(&mut self) {
        self.values.clear();
    }
}

impl PromptsState {
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn iter(&self) -> std::slice::Iter<Option<Ansi>> {
        self.values.iter()
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    #[allow(dead_code)]
    pub fn get(&self, index: usize) -> Option<&Ansi> {
        self.values.get(index).map_or(None, |v| v.as_ref())
    }

    pub fn set_index(&mut self, index: usize, content: Ansi) {
        while self.values.len() <= index {
            self.values.push(None);
        }
        self.values[index] = Some(content);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_index_first() {
        let mut state = PromptsState::default();
        state.set_index(0, Ansi::from("grayskull"));
        assert_eq!(state.len(), 1);
    }

    #[test]
    fn set_after_first() {
        let mut state = PromptsState::default();
        state.set_index(2, Ansi::from("grayskull"));
        assert_eq!(state.len(), 3);
        assert_eq!(state.get(0), None);
        assert_eq!(state.get(1), None);
        assert_eq!(state.get(2), Some(&Ansi::from("grayskull")));
    }
}
