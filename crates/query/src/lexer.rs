use logos::{Logos, Lexer};

/// Raw tokens used internally by logos
#[derive(Logos, Debug, PartialEq, Clone)]
#[logos(skip r"[ \t\n\f]+")]
enum RawToken {
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

    #[regex(r#""([^"\\]|\\.)*""#, quoted_text_inner_string)]
    QuotedText(String),

    #[regex(r#"[^ \t\n\f:"()!&|]+"#, |lex| lex.slice().to_string())]
    Text(String),
}

fn quoted_text_inner_string(lex: &mut Lexer<RawToken>) -> String {
    let slice = lex.slice();
    slice.get(1..slice.len() - 1).unwrap().to_string()
}

/// Value tokens - used after `:` where operators are treated as text
#[derive(Logos, Debug, PartialEq, Clone)]
enum ValueToken {
    #[regex(r#""([^"\\]|\\.)*""#)]
    Quoted,

    #[regex(r#"[^ \t\n\f"]+"#)]
    Text,

    #[regex(r"[ \t\n\f]+")]
    Whitespace,
}

/// The public token type
#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    And,
    Or,
    Not,
    Colon,
    LParen,
    RParen,
    QuotedText(String),
    Text(String),
}

/// A context-aware query lexer
pub struct QueryLexer<'source> {
    lexer: Lexer<'source, RawToken>,
    after_colon: bool,
    current_span: std::ops::Range<usize>,
}

impl<'source> QueryLexer<'source> {
    pub fn new(source: &'source str) -> Self {
        Self {
            lexer: RawToken::lexer(source),
            after_colon: false,
            current_span: 0..0,
        }
    }

    /// Get the span of the last yielded token
    pub fn span(&self) -> std::ops::Range<usize> {
        self.current_span.clone()
    }

    /// Get the slice of the last yielded token
    pub fn slice(&self) -> &'source str {
        &self.lexer.source()[self.current_span.clone()]
    }

    /// Returns an iterator that yields (Result<Token, ()>, Range<usize>)
    pub fn spanned(self) -> SpannedQueryLexer<'source> {
        SpannedQueryLexer { lexer: self }
    }
}

impl<'source> Iterator for QueryLexer<'source> {
    type Item = Result<Token, ()>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.after_colon {
            self.after_colon = false;
            let mut value_lexer: Lexer<'source, ValueToken> = self.lexer.clone().morph();

            let result = match value_lexer.next()? {
                Ok(ValueToken::Quoted) => {
                    let slice = value_lexer.slice();
                    let s = slice.get(1..slice.len() - 1).unwrap().to_string();
                    self.current_span = value_lexer.span();
                    self.lexer = value_lexer.morph();
                    Some(Ok(Token::QuotedText(s.into())))
                }
                Ok(ValueToken::Text) => {
                    let s = value_lexer.slice().to_string();
                    self.current_span = value_lexer.span();
                    self.lexer = value_lexer.morph();
                    Some(Ok(Token::Text(s)))
                }
                Ok(ValueToken::Whitespace) => {
                    // No value after colon, continue in normal mode
                    self.lexer = value_lexer.morph();
                    self.next()
                }
                Err(_) => {
                    self.current_span = value_lexer.span();
                    self.lexer = value_lexer.morph();
                    Some(Err(()))
                }
            };
            return result;
        }

        let result = self.lexer.next()?;
        self.current_span = self.lexer.span();

        match result {
            Ok(RawToken::Colon) => {
                self.after_colon = true;
                Some(Ok(Token::Colon))
            }
            Ok(RawToken::And) => Some(Ok(Token::And)),
            Ok(RawToken::Or) => Some(Ok(Token::Or)),
            Ok(RawToken::Not) => Some(Ok(Token::Not)),
            Ok(RawToken::LParen) => Some(Ok(Token::LParen)),
            Ok(RawToken::RParen) => Some(Ok(Token::RParen)),
            Ok(RawToken::QuotedText(s)) => Some(Ok(Token::QuotedText(s))),
            Ok(RawToken::Text(s)) => Some(Ok(Token::Text(s))),
            Err(_) => Some(Err(())),
        }
    }
}

/// Iterator adapter that yields tokens with their spans
pub struct SpannedQueryLexer<'source> {
    lexer: QueryLexer<'source>,
}

impl<'source> Iterator for SpannedQueryLexer<'source> {
    type Item = (Result<Token, ()>, std::ops::Range<usize>);

    fn next(&mut self) -> Option<Self::Item> {
        let token = self.lexer.next()?;
        let span = self.lexer.span();
        Some((token, span))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_quoted_text_inner_string0() {
        let input = r#"r:"a""#;
        let tokens: Vec<_> = QueryLexer::new(input).collect();
        assert_eq!(tokens, vec![
            Ok(Token::Text("r".into())),
            Ok(Token::Colon),
            Ok(Token::QuotedText("a".into()))
        ]);
    }

    #[test]
    fn test_quoted_text_inner_string1() {
        let input = r#""a""#;
        let tokens: Vec<_> = QueryLexer::new(input).collect();
        assert_eq!(tokens, vec![
            Ok(Token::QuotedText("a".into()))
        ]);
    }


    #[test]
    fn test_op_after_colon() {
        let input = r#"r:(1AND)"#;
        let tokens: Vec<_> = QueryLexer::new(input).collect();
        assert_eq!(tokens, vec![
            Ok(Token::Text("r".into())),
            Ok(Token::Colon),
            Ok(Token::Text(r#"(1AND)"#.into()))
        ]);
    }

    #[test]
    fn test_glob_with_exclamation() {
        let input = r#"glob:!*.py"#;
        let tokens: Vec<_> = QueryLexer::new(input).collect();
        assert_eq!(tokens, vec![
            Ok(Token::Text("glob".into())),
            Ok(Token::Colon),
            Ok(Token::Text("!*.py".into())),
        ]);
    }

    #[test]
    fn test_spanned_simple() {
        let input = "foo:bar";
        let tokens: Vec<(Token, std::ops::Range<usize>)> = QueryLexer::new(input)
            .spanned()
            .filter_map(|(result, span)| result.ok().map(|t| (t, span)))
            .collect();

        assert_eq!(tokens, vec![
            (Token::Text("foo".into()), 0..3),
            (Token::Colon, 3..4),
            (Token::Text("bar".into()), 4..7),
        ]);
    }

    #[test]
    fn test_spanned_with_operators() {
        let input = "foo AND bar";
        let tokens: Vec<(Token, std::ops::Range<usize>)> = QueryLexer::new(input)
            .spanned()
            .filter_map(|(result, span)| result.ok().map(|t| (t, span)))
            .collect();

        assert_eq!(tokens, vec![
            (Token::Text("foo".into()), 0..3),
            (Token::And, 4..7),
            (Token::Text("bar".into()), 8..11),
        ]);
    }

    #[test]
    fn test_spanned_complex() {
        let input = r#"type:!*.rs || name:"test""#;
        let tokens: Vec<(Token, std::ops::Range<usize>)> = QueryLexer::new(input)
            .spanned()
            .filter_map(|(result, span)| result.ok().map(|t| (t, span)))
            .collect();

        assert_eq!(tokens, vec![
            (Token::Text("type".into()), 0..4),
            (Token::Colon, 4..5),
            (Token::Text("!*.rs".into()), 5..10),
            (Token::Or, 11..13),
            (Token::Text("name".into()), 14..18),
            (Token::Colon, 18..19),
            (Token::QuotedText("test".into()), 19..25),
        ]);
    }

    #[test]
    fn test_spanned_and_as_value() {
        let input = "field:AND";
        let tokens: Vec<(Token, std::ops::Range<usize>)> = QueryLexer::new(input)
            .spanned()
            .filter_map(|(result, span)| result.ok().map(|t| (t, span)))
            .collect();

        assert_eq!(tokens, vec![
            (Token::Text("field".into()), 0..5),
            (Token::Colon, 5..6),
            (Token::Text("AND".into()), 6..9),
        ]);
    }

    #[test]
    fn test_spanned_verify_slices() {
        let input = "glob:!*.py AND ext:rs";
        
        let tokens: Vec<(Token, std::ops::Range<usize>)> = QueryLexer::new(input)
            .spanned()
            .filter_map(|(result, span)| result.ok().map(|t| (t, span)))
            .collect();

        // Verify that spans correctly index into original input
        for (token, span) in &tokens {
            let slice = &input[span.clone()];
            match token {
                Token::Text(s) | Token::QuotedText(s) => {
                    assert_eq!(slice, s);
                }
                Token::And => assert_eq!(slice, "AND"),
                Token::Or => assert!(slice == "OR" || slice == "||"),
                Token::Not => assert!(slice == "NOT" || slice == "!"),
                Token::Colon => assert_eq!(slice, ":"),
                Token::LParen => assert_eq!(slice, "("),
                Token::RParen => assert_eq!(slice, ")"),
            }
        }
    }
}
