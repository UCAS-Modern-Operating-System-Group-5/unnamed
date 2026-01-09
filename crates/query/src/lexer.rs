pub use logos::Logos;

#[derive(Logos, Debug, PartialEq, Clone)]
#[logos(skip r"[ \t\n\f]+")]
pub enum Token {
    #[token("AND")]
    #[token("&&")]
    And,

    #[token("OR")]
    #[token("||")]
    Or,

    #[token("NOT")]
    #[token("!")]
    Not,

    #[token(":")]
    Colon,

    #[token("(")]
    LParen,

    #[token(")")]
    RParen,

    #[regex(r#""([^"\\]|\\.)*""#, |lex| lex.slice().to_string())]
    QuotedText(String),

    /// Generic Words/Identifiers
    #[regex(r#"[^ \t\n\f:"()!&|]+"#, |lex| lex.slice().to_string())]
    Text(String),
}

#[cfg(test)]
mod test {
    use super::*;

    use rstest::rstest;

    #[rstest]
    #[case("AND", Token::And)]
    #[case("&&", Token::And)]
    #[case("OR", Token::Or)]
    #[case("||", Token::Or)]
    #[case("NOT", Token::Not)]
    #[case("!", Token::Not)]
    #[case(":", Token::Colon)]
    #[case("(", Token::LParen)]
    #[case(")", Token::RParen)]
    #[case("\"okey docky\"", Token::QuotedText(r#""okey docky""#.into()))]
    #[case(r#""okey docky\"""#, Token::QuotedText(r#""okey docky\"""#.into()))]
    fn test_lex_single(#[case] input: &str, #[case] token: Token) {
        let mut lex = Token::lexer(input);
        assert_eq!(lex.next(), Some(Ok(token)));
    }

    #[test]
    fn test_lex_sample0() {
        let input = r#"(!r:*.rs || regexp:"*.py") root:/etc"#;
        let mut lex = Token::lexer(input);
        assert_eq!(lex.next(), Some(Ok(Token::LParen)));
        assert_eq!(lex.next(), Some(Ok(Token::Not)));
        assert_eq!(lex.next(), Some(Ok(Token::Text("r".into()))));
        assert_eq!(lex.next(), Some(Ok(Token::Colon)));
        assert_eq!(lex.next(), Some(Ok(Token::Text("*.rs".into()))));
        assert_eq!(lex.next(), Some(Ok(Token::Or)));
        assert_eq!(lex.next(), Some(Ok(Token::Text("regexp".into()))));
        assert_eq!(lex.next(), Some(Ok(Token::Colon)));
        assert_eq!(lex.next(), Some(Ok(Token::QuotedText(r#""*.py""#.into()))));
        assert_eq!(lex.next(), Some(Ok(Token::RParen)));
        assert_eq!(lex.next(), Some(Ok(Token::Text("root".into()))));
        assert_eq!(lex.next(), Some(Ok(Token::Colon)));
        assert_eq!(lex.next(), Some(Ok(Token::Text("/etc".into()))));
    }
}
