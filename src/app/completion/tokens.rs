use regex::{Matches, Regex};

pub struct Tokens<'a> {
    regex: Regex,
    input: &'a str,
}

impl<'a> Tokens<'a> {
    /// Select only "significant" tokens
    pub fn significant_from(text: &'a str) -> Tokens<'a> {
        let regex = Regex::new(r"(\w{3,}([']\w+)?)").unwrap();
        Tokens { regex, input: text }
    }

    /// Select all tokens
    pub fn from(text: &'a str) -> Tokens<'a> {
        let regex = Regex::new(r"(\w+([']\w+)?)").unwrap();
        Tokens { regex, input: text }
    }

    pub fn iter(&self) -> TokensIter {
        TokensIter {
            matches: self.regex.find_iter(self.input),
        }
    }
}

impl<'a> IntoIterator for &'a Tokens<'a> {
    type Item = String;

    type IntoIter = TokensIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

pub struct TokensIter<'a> {
    matches: Matches<'a, 'a>,
}

impl Iterator for TokensIter<'_> {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        self.matches.next().map(|m| m.as_str().to_lowercase())
    }
}

#[cfg(test)]
mod test {
    use crate::app::completion::tokens::Tokens;

    impl Tokens<'_> {
        fn into_vec(self) -> Vec<String> {
            self.iter().collect()
        }
    }

    #[test]
    fn test_empty() {
        assert_eq!(Tokens::from("").into_vec(), Vec::<String>::default());
        assert_eq!(
            Tokens::significant_from("").into_vec(),
            Vec::<String>::default()
        );
    }

    #[test]
    fn test_symbols() {
        assert_eq!(Tokens::from("( *$ ][").into_vec(), Vec::<String>::default());
        assert_eq!(
            Tokens::significant_from("( *$ ][").into_vec(),
            Vec::<String>::default()
        );
    }

    #[test]
    fn test_words() {
        assert_eq!(
            Tokens::from("You can't (take)").into_vec(),
            ["you", "can't", "take"]
        );
    }

    #[test]
    fn test_significant_words() {
        assert_eq!(
            Tokens::from("it's no big deal").into_vec(),
            ["it's", "no", "big", "deal"]
        );
        assert_eq!(
            Tokens::significant_from("it's no big deal").into_vec(),
            ["big", "deal"]
        );
    }
}
