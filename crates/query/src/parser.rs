use chumsky::{input::ValueInput, prelude::*};

use crate::{QueryLexer, lexer::Token};

pub type Span = SimpleSpan;
pub type Spanned<T> = (T, Span);

#[derive(Debug, Clone, PartialEq)]
pub enum ParsedQuery {
    /// A single search term: `field:value` or just `value`
    Term(ParsedTerm),

    /// Logical And
    And(Vec<Spanned<ParsedQuery>>),

    /// Logical Or
    Or(Vec<Spanned<ParsedQuery>>),

    /// Logical Not
    Not(Box<Spanned<ParsedQuery>>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedTerm {
    pub field: Option<Spanned<String>>,
    pub value: Spanned<ParsedTermValue>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParsedTermValue {
    /// Plain text e.g. `abc`
    Text(String),

    /// Quoted text (Not includes quotes) e.g. `a\"b c`
    QuotedText(String),
}

impl ParsedTermValue {
    pub fn raw_str(&self) -> &str {
        match self {
            ParsedTermValue::Text(s) => s,
            ParsedTermValue::QuotedText(s) => s,
        }
    }

    /// Return the string inside the value with escaped quotes interpretation for
    /// QuotedText. e.g. `a\"b` -> `ab`.
    pub fn to_string(&self) -> String {
        match self {
            ParsedTermValue::Text(s) => s.into(),
            ParsedTermValue::QuotedText(s) => s.replace(r#"\""#, r#"""#),
        }
    }
}

/// Parser for Lucene/Google-style queries
///
/// Grammar(lower to higher priority):
/// ```text
/// query       := or_expr
/// or_expr     := and_expr (OR and_expr)*
/// and_expr    := not_expr ((AND)? not_expr)*
/// not_expr    := NOT* atom
/// atom        := term | '(' query ')'
/// term        := (field ':')? value
/// value       := Text | QuotedText
/// ```
pub fn parser<'tokens, I>()
-> impl Parser<'tokens, I, Spanned<ParsedQuery>, extra::Err<Rich<'tokens, Token>>>
where
    I: ValueInput<'tokens, Token = Token, Span = SimpleSpan>,
{
    recursive(|query| {
        let field_with_span =
            select! { Token::Text(s) => s }.map_with(|s, e| (s, e.span()));

        let value_with_span = select! {
            Token::Text(s) => ParsedTermValue::Text(s),
            Token::QuotedText(s) => ParsedTermValue::QuotedText(s),
        }
        .map_with(|v, e| (v, e.span()));

        let term = field_with_span
            .clone()
            .then(
                just(Token::Colon)
                    .ignore_then(value_with_span.clone())
                    .or_not(),
            )
            .map(|(field_spanned, value_opt)| match value_opt {
                Some(value) => ParsedTerm {
                    field: Some(field_spanned),
                    value,
                },
                None => {
                    let (text, span) = field_spanned;
                    ParsedTerm {
                        field: None,
                        value: (ParsedTermValue::Text(text), span),
                    }
                }
            })
            .or(
                select! { Token::QuotedText(s) => ParsedTermValue::QuotedText(s) }
                    .map_with(|v, e| ParsedTerm {
                        field: None,
                        value: (v, e.span()),
                    }),
            )
            .map(ParsedQuery::Term)
            .map_with(|q, e| (q, e.span()));

        let atom = term.or(query
            .clone()
            .delimited_by(just(Token::LParen), just(Token::RParen)));

        let not_expr = just(Token::Not).map_with(|_, e| e.span()).repeated().foldr(
            atom,
            |not_span: SimpleSpan, (q, q_span): Spanned<ParsedQuery>| {
                let combined_span = (not_span.start..q_span.end).into();
                (ParsedQuery::Not(Box::new((q, q_span))), combined_span)
            },
        );

        let and_expr = not_expr.clone().foldl(
            choice((
                just(Token::And).ignore_then(not_expr.clone()),
                not_expr.clone(),
            ))
            .repeated(),
            |lhs: Spanned<ParsedQuery>, rhs: Spanned<ParsedQuery>| {
                let span = (lhs.1.start..rhs.1.end).into();
                match lhs {
                    (ParsedQuery::And(mut v), _) => {
                        v.push(rhs);
                        (ParsedQuery::And(v), span)
                    }
                    _ => (ParsedQuery::And(vec![lhs, rhs]), span),
                }
            },
        );

        and_expr.clone().foldl(
            just(Token::Or).ignore_then(and_expr).repeated(),
            |lhs: Spanned<ParsedQuery>, rhs: Spanned<ParsedQuery>| {
                let span = (lhs.1.start..rhs.1.end).into();
                match lhs {
                    (ParsedQuery::Or(mut v), _) => {
                        v.push(rhs);
                        (ParsedQuery::Or(v), span)
                    }
                    _ => (ParsedQuery::Or(vec![lhs, rhs]), span),
                }
            },
        )
    })
}

/// Helper function to parse a query string and return the result
pub fn parse_query(input: &str) -> Result<Spanned<ParsedQuery>, Vec<Rich<'_, Token>>> {
    use chumsky::input::Stream;

    let token_iter = QueryLexer::new(input)
        .spanned()
        .map(|(tok, span)| (tok.expect("Lexer error"), SimpleSpan::from(span)));

    let token_stream = Stream::from_iter(token_iter)
        .map((0..input.len()).into(), |(t, s): (_, _)| (t, s));

    parser().parse(token_stream).into_result()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_term() {
        let result = parse_query("hello").unwrap();
        assert!(matches!(result.0, ParsedQuery::Term(_)));
        if let ParsedQuery::Term(term) = &result.0 {
            assert!(term.field.is_none());
            assert_eq!(term.value.0.raw_str(), "hello");
        }
    }

    #[test]
    fn test_quoted_term() {
        let result = parse_query(r#""hello world""#).unwrap();
        if let ParsedQuery::Term(term) = &result.0 {
            assert!(term.field.is_none());
            assert_eq!(term.value.0.raw_str(), "hello world");
        }
    }

    #[test]
    fn test_field_term() {
        let result = parse_query("title:hello").unwrap();
        if let ParsedQuery::Term(term) = &result.0 {
            assert_eq!(term.field.as_ref().unwrap().0, "title");
            assert_eq!(term.value.0.raw_str(), "hello");
        }
    }

    #[test]
    fn test_complex_query_with_spans() {
        let input = r#"(!glob:!*.rs || regexp:"*.py") root:/etc"#;
        let result = parse_query(input).unwrap();

        // The overall span starts at 1 (the '!' inside the parens) because
        // delimited_by doesn't include the delimiters in the inner expr's span
        assert_eq!(result.1.start, 1);
        assert_eq!(result.1.end, 40); // end of "/etc"

        // Top level should be AND
        let ParsedQuery::And(and_items) = &result.0 else {
            panic!("Expected And at top level");
        };
        assert_eq!(and_items.len(), 2);

        let (or_query, or_span) = &and_items[0];
        assert_eq!(or_span.start, 1); // '!'
        assert_eq!(or_span.end, 29); // '"*.py"'

        let ParsedQuery::Or(or_items) = or_query else {
            panic!("Expected Or as first And item");
        };
        assert_eq!(or_items.len(), 2);

        let (not_query, not_span) = &or_items[0];
        assert_eq!(not_span.start, 1); // '!'
        assert_eq!(not_span.end, 12); // "*.rs"

        let ParsedQuery::Not(not_inner) = not_query else {
            panic!("Expected Not");
        };

        let (term_query, term_span) = not_inner.as_ref();
        assert_eq!(term_span.start, 2); // 'r'
        assert_eq!(term_span.end, 12); // "*.rs"

        let ParsedQuery::Term(term) = term_query else {
            panic!("Expected Term inside Not");
        };

        // Field "glob"
        let (field_name, field_span) = term.field.as_ref().unwrap();
        assert_eq!(field_name, "glob");
        assert_eq!(field_span.start, 2);
        assert_eq!(field_span.end, 6);

        // Value "!*.rs"
        assert_eq!(term.value.0.raw_str(), "!*.rs");
        assert_eq!(term.value.1.start, 7);
        assert_eq!(term.value.1.end, 12);

        // Second OR item: regexp:"*.py"
        let (regexp_query, regexp_span) = &or_items[1];
        assert_eq!(regexp_span.start, 16); // 'r' of regexp
        assert_eq!(regexp_span.end, 29); // end of '"*.py"'

        let ParsedQuery::Term(regexp_term) = regexp_query else {
            panic!("Expected Term for regexp");
        };

        let (regexp_field, regexp_field_span) = regexp_term.field.as_ref().unwrap();
        assert_eq!(regexp_field, "regexp");
        assert_eq!(regexp_field_span.start, 16);
        assert_eq!(regexp_field_span.end, 22);

        // Quoted text
        assert_eq!(regexp_term.value.0.raw_str(), "*.py");
        // The span includes the quotes
        assert_eq!(regexp_term.value.1.start, 23);
        assert_eq!(regexp_term.value.1.end, 29);

        // Second AND item: root:/etc
        let (root_query, root_span) = &and_items[1];
        assert_eq!(root_span.start, 31); // 'r' of root
        assert_eq!(root_span.end, 40); // end of "/etc"

        let ParsedQuery::Term(root_term) = root_query else {
            panic!("Expected Term for root");
        };

        let (root_field, root_field_span) = root_term.field.as_ref().unwrap();
        assert_eq!(root_field, "root");
        assert_eq!(root_field_span.start, 31);
        assert_eq!(root_field_span.end, 35);

        assert_eq!(root_term.value.0.raw_str(), "/etc");
        assert_eq!(root_term.value.1.start, 36);
        assert_eq!(root_term.value.1.end, 40);
    }

    #[test]
    fn test_span_info_preserved() {
        let input = "foo:bar";
        let result = parse_query(input).unwrap();
        if let ParsedQuery::Term(term) = &result.0 {
            let (field_name, field_span) = term.field.as_ref().unwrap();
            assert_eq!(field_name, "foo");
            assert_eq!(field_span.start, 0);
            assert_eq!(field_span.end, 3);

            let (_, value_span) = &term.value;
            assert_eq!(value_span.start, 4);
            assert_eq!(value_span.end, 7);
        }
    }

    #[test]
    fn test_implicit_and() {
        let result = parse_query("foo bar").unwrap();
        assert!(matches!(&result.0, ParsedQuery::And(items) if items.len() == 2));
    }

    #[test]
    fn test_explicit_and() {
        let result = parse_query("foo AND bar").unwrap();
        assert!(matches!(&result.0, ParsedQuery::And(items) if items.len() == 2));
    }

    #[test]
    fn test_or() {
        let result = parse_query("foo OR bar").unwrap();
        assert!(matches!(&result.0, ParsedQuery::Or(items) if items.len() == 2));
    }

    #[test]
    fn test_not() {
        let result = parse_query("NOT foo").unwrap();
        assert!(matches!(&result.0, ParsedQuery::Not(_)));
    }

    #[test]
    fn test_precedence() {
        // "a OR b c" should parse as "a OR (b AND c)"
        let result = parse_query("a OR b c").unwrap();
        if let ParsedQuery::Or(items) = &result.0 {
            assert_eq!(items.len(), 2);
            assert!(matches!(&items[0].0, ParsedQuery::Term(_)));
            assert!(matches!(&items[1].0, ParsedQuery::And(_)));
        } else {
            panic!("Expected Or at top level");
        }
    }

    #[test]
    fn test_nested_parens() {
        let result = parse_query("((a))").unwrap();
        assert!(matches!(&result.0, ParsedQuery::Term(_)));
    }

    #[test]
    fn test_double_not() {
        let result = parse_query("NOT NOT foo").unwrap();
        if let ParsedQuery::Not(inner) = &result.0 {
            assert!(matches!(&inner.0, ParsedQuery::Not(_)));
        } else {
            panic!("Expected Not at top level");
        }
    }

    #[test]
    fn test_complex_query0() {
        let result = parse_query(r#"(!r:*.rs || regexp:"*.py") root:/etc"#).unwrap();

        // Should be And at top level
        if let ParsedQuery::And(items) = &result.0 {
            assert_eq!(items.len(), 2);
            assert!(matches!(&items[0].0, ParsedQuery::Or(_)));
            assert!(matches!(&items[1].0, ParsedQuery::Term(_)));
        } else {
            panic!("Expected And at top level");
        }
    }

    #[test]
    fn test_complex_query1() {
        let result =
            parse_query("(size:>100MB AND mtime:>30d) OR (name:*.tmp AND mtime:>7d)");
        assert!(result.is_ok());
    }
}
