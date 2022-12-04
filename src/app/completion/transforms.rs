#[derive(Clone, Copy, Debug, PartialEq)]
enum CharTransform {
    Upper,
    Lower,
    Nop,
}

impl CharTransform {
    fn from(ch: char) -> CharTransform {
        if !ch.is_ascii_alphabetic() {
            CharTransform::Nop
        } else if Some(ch) == char::to_lowercase(ch).next() {
            CharTransform::Lower
        } else {
            CharTransform::Upper
        }
    }

    fn transform(&self, ch: char) -> Option<char> {
        match self {
            CharTransform::Upper => char::to_uppercase(ch).next(),
            CharTransform::Lower => char::to_lowercase(ch).next(),
            CharTransform::Nop => Some(ch),
        }
    }
}

impl From<Option<char>> for CharTransform {
    fn from(option: Option<char>) -> Self {
        match option {
            Some(ch) => Self::from(ch),
            None => CharTransform::Nop,
        }
    }
}

#[derive(Clone, Copy)]
pub struct WordTransform {
    first: CharTransform,
    rest: CharTransform,
}

impl WordTransform {
    pub fn matching_word<T: AsRef<str>>(input: T) -> WordTransform {
        let s: &str = input.as_ref();
        let mut chars = s.chars();

        let first: CharTransform = chars.next().into();
        let rest: CharTransform = chars.next().into();

        WordTransform { first, rest }
    }

    pub fn transform<T: AsRef<str>>(&self, word: T) -> String {
        let input = word.as_ref();
        input
            .char_indices()
            .filter_map(|(i, ch)| {
                if i == 0 {
                    self.first.transform(ch)
                } else {
                    self.rest.transform(ch)
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matching_word_test() {
        let title = WordTransform::matching_word("Grayskull");
        assert_eq!(title.first, CharTransform::Upper);
        assert_eq!(title.rest, CharTransform::Lower);

        let lower = WordTransform::matching_word("sword");
        assert_eq!(lower.first, CharTransform::Lower);
        assert_eq!(lower.rest, CharTransform::Lower);

        let upper = WordTransform::matching_word("HONOR");
        assert_eq!(upper.first, CharTransform::Upper);
        assert_eq!(upper.rest, CharTransform::Upper);
    }

    #[test]
    fn transform_test() {
        let transform = WordTransform::matching_word("Grayskull");
        assert_eq!(transform.transform("adORa"), "Adora");
    }
}
