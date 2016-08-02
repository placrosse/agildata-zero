
#[derive(Debug,PartialEq)]
pub enum Token {
    Whitespace,
    Keyword(String),
    Identifier(String),
    LiteralString(String),
    LiteralLong(String),
    Operator(String),
    Comma
}

pub trait Tokenizer {
    fn tokenize(&self) -> Result<Vec<Token>, &'static str>;
}

impl Tokenizer for String {

    fn tokenize(&self) -> Result<Vec<Token>, &'static str> {
        Ok(vec![Token::Keyword("TEST".to_string())])
    }

}

#[cfg(test)]
mod tests {
    use super::{Token, Tokenizer};
    use super::Token::*;

    #[test]
    fn simple_tokenize() {
        assert_eq!(
            vec![Keyword("SELECT".to_string()),
                LiteralLong("1".to_string()),
                Operator("+".to_string()),
                LiteralLong("1".to_string())],
            String::from("SELECT 1 + 1").tokenize().unwrap()
        );
    }

}
