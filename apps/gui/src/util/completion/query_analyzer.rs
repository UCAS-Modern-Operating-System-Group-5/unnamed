// FIXME support non-ascii characters
// thread 'tokio-runtime-worker' (296782) panicked at apps/gui/src/util/completion/query_analyzer.rs:35:37:
// byte index 32 is not a char boundary; it is inside '结' (bytes 31..34) of `root:~/Documents/archive/星火结项材料.

use query::lexer::{Token, QueryLexer};

#[derive(Debug, Clone)]
pub enum CompletionContext {
    /// Empty query or at start
    Empty,
    /// Just typed a field name, expecting colon or more text
    /// e.g., "roo|" could become "root:"
    PartialFieldOrTerm { text: String, start_pos: usize },
    /// After "field:", expecting value
    /// e.g., "root:|" or "root:/etc|"
    FieldValue {
        field: String,
        value: String,
        value_start: usize,
    },
    /// After a complete term, could type operator or new term  
    /// e.g., "root:/etc |"
    AfterTerm,
    /// After AND/OR/NOT, expecting term
    AfterOperator,
    /// Inside parentheses
    InGroup { depth: usize },
    /// Inside quoted string - no completion
    InQuotedString,
}

pub struct QueryAnalyzer;

// We here only use lexer to do simple analyzer
// If parser is implemented excellently, then later we can switch to a parser implementation
impl QueryAnalyzer {
    /// Analyze query and cursor position to determine completion context
    pub fn analyze(query: &str, cursor_pos: usize) -> CompletionContext {
        let query_to_cursor = &query[..cursor_pos.min(query.len())];

        if query_to_cursor.trim().is_empty() {
            return CompletionContext::Empty;
        }

        if Self::has_unclosed_quote(query_to_cursor) {
            return CompletionContext::InQuotedString;
        }

        let lexer = QueryLexer::new(query_to_cursor);
        let tokens: Vec<(Token, std::ops::Range<usize>)> = lexer
            .spanned()
            .filter_map(|(result, span)| result.ok().map(|t| (t, span)))
            .collect();

        Self::analyze_tokens(query_to_cursor, &tokens)
    }

    fn has_unclosed_quote(s: &str) -> bool {
        let mut in_quote = false;
        let mut prev_char = ' ';
        for c in s.chars() {
            if c == '"' && prev_char != '\\' {
                in_quote = !in_quote;
            }
            prev_char = c;
        }
        in_quote
    }

    fn analyze_tokens(
        query: &str,
        tokens: &[(Token, std::ops::Range<usize>)],
    ) -> CompletionContext {
        if tokens.is_empty() {
            // Has text but no valid tokens - probably partial text
            let text = query.trim();
            return CompletionContext::PartialFieldOrTerm {
                text: text.to_string(),
                start_pos: query.len() - text.len(),
            };
        }

        let (last_token, last_span) = tokens.last().unwrap();
        // let ends_at_cursor = last_span.end == query.len();
        let ends_with_space = query.ends_with(' ');

        match last_token {
            Token::Colon => {
                // "field:|" - need to find the field name
                if tokens.len() >= 2 {
                    if let (Token::Text(field), _) = &tokens[tokens.len() - 2] {
                        return CompletionContext::FieldValue {
                            field: field.clone(),
                            value: String::new(),
                            value_start: last_span.end,
                        };
                    }
                }
                CompletionContext::AfterTerm
            }

            Token::Text(text) => {
                // Check if previous token was colon (field:value pattern)
                if tokens.len() >= 2 {
                    let (prev_token, _) = &tokens[tokens.len() - 2];
                    if *prev_token == Token::Colon && tokens.len() >= 3 {
                        if let (Token::Text(field), _) = &tokens[tokens.len() - 3] {
                            // TODO check `ends_with_space`?
                            return CompletionContext::FieldValue {
                                field: field.clone(),
                                value: text.clone(),
                                value_start: last_span.start,
                            };
                        }
                    }
                }

                if ends_with_space {
                    CompletionContext::AfterTerm
                } else {
                    CompletionContext::PartialFieldOrTerm {
                        text: text.clone(),
                        start_pos: last_span.start,
                    }
                }
            }

            Token::QuotedText(_) => {
                if ends_with_space {
                    CompletionContext::AfterTerm
                } else {
                    CompletionContext::InQuotedString
                }
            }

            Token::And | Token::Or | Token::Not => CompletionContext::AfterOperator,

            Token::LParen => {
                let depth = tokens
                    .iter()
                    .filter(|(t, _)| matches!(t, Token::LParen))
                    .count()
                    - tokens
                        .iter()
                        .filter(|(t, _)| matches!(t, Token::RParen))
                        .count();
                CompletionContext::InGroup { depth }
            }

            Token::RParen => CompletionContext::AfterTerm,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // ==================== Empty Context ====================
    #[rstest]
    #[case("", 0)]
    #[case("   ", 0)]
    #[case("   ", 1)]
    #[case("   ", 3)]
    #[case("\t\n ", 3)]
    #[case("hello", 0)] // cursor at start, nothing to analyze
    fn test_empty_context(#[case] query: &str, #[case] cursor_pos: usize) {
        assert!(matches!(
            QueryAnalyzer::analyze(query, cursor_pos),
            CompletionContext::Empty
        ));
    }

    // ==================== InQuotedString Context ====================
    #[rstest]
    #[case(r#"""#, 1)]
    #[case(r#""hello"#, 6)]
    #[case(r#"root:""#, 6)]
    #[case(r#"root:"test"#, 10)]
    #[case(r#"hello "world"#, 12)]
    #[case(r#"test "unclosed"#, 15)]
    #[case(r#""has \"escaped"#, 14)] // escaped quote, still unclosed
    fn test_in_quoted_string(#[case] query: &str, #[case] cursor_pos: usize) {
        assert!(matches!(
            QueryAnalyzer::analyze(query, cursor_pos),
            CompletionContext::InQuotedString
        ));
    }

    // ==================== PartialFieldOrTerm Context ====================
    #[rstest]
    #[case("r", 1, "r", 0)]
    #[case("roo", 3, "roo", 0)]
    #[case("root", 4, "root", 0)]
    #[case("hello AND wor", 13, "wor", 10)]
    #[case("(test", 5, "test", 1)]
    #[case("NOT foo", 7, "foo", 4)]
    fn test_partial_field_or_term(
        #[case] query: &str,
        #[case] cursor_pos: usize,
        #[case] expected_text: &str,
        #[case] expected_start: usize,
    ) {
        match QueryAnalyzer::analyze(query, cursor_pos) {
            CompletionContext::PartialFieldOrTerm { text, start_pos } => {
                assert_eq!(text, expected_text);
                assert_eq!(start_pos, expected_start);
            }
            other => panic!("Expected PartialFieldOrTerm, got {:?}", other),
        }
    }

    // ==================== FieldValue Context (empty value) ====================
    #[rstest]
    #[case("root:", 5, "root", "", 5)]
    #[case("type:", 5, "type", "", 5)]
    #[case("path:", 5, "path", "", 5)]
    #[case("hello AND root:", 15, "root", "", 15)]
    #[case("(root:", 6, "root", "", 6)]
    fn test_field_value_empty(
        #[case] query: &str,
        #[case] cursor_pos: usize,
        #[case] expected_field: &str,
        #[case] expected_value: &str,
        #[case] expected_start: usize,
    ) {
        match QueryAnalyzer::analyze(query, cursor_pos) {
            CompletionContext::FieldValue {
                field,
                value,
                value_start,
            } => {
                assert_eq!(field, expected_field);
                assert_eq!(value, expected_value);
                assert_eq!(value_start, expected_start);
            }
            other => panic!("Expected FieldValue, got {:?}", other),
        }
    }

    // ==================== FieldValue Context (with value) ====================
    #[rstest]
    #[case("root:/", 6, "root", "/", 5)]
    #[case("root:/etc", 9, "root", "/etc", 5)]
    #[case("root:/etc ", 10, "root", "/etc", 5)] // trailing space still FieldValue (see TODO in code)
    #[case("type:rs", 7, "type", "rs", 5)]
    #[case("path:/home/user", 15, "path", "/home/user", 5)]
    #[case("ext:*.rs", 8, "ext", "*.rs", 4)]
    #[case("hello AND root:/etc", 19, "root", "/etc", 15)]
    fn test_field_value_with_content(
        #[case] query: &str,
        #[case] cursor_pos: usize,
        #[case] expected_field: &str,
        #[case] expected_value: &str,
        #[case] expected_start: usize,
    ) {
        match QueryAnalyzer::analyze(query, cursor_pos) {
            CompletionContext::FieldValue {
                field,
                value,
                value_start,
            } => {
                assert_eq!(field, expected_field);
                assert_eq!(value, expected_value);
                assert_eq!(value_start, expected_start);
            }
            other => panic!("Expected FieldValue, got {:?}", other),
        }
    }

    // ==================== AfterTerm Context ====================
    #[rstest]
    #[case("hello ", 6)]
    #[case("test ", 5)]
    #[case(r#""quoted" "#, 10)] // quoted string with trailing space
    #[case("(hello)", 7)] // after closing paren
    #[case("(a) ", 4)]
    #[case("(()  (", 5)]
    #[case(":", 1)] // lone colon treated as AfterTerm
    fn test_after_term(#[case] query: &str, #[case] cursor_pos: usize) {
        assert!(matches!(
            QueryAnalyzer::analyze(query, cursor_pos),
            CompletionContext::AfterTerm
        ));
    }

    // ==================== AfterOperator Context ====================
    #[rstest]
    #[case("AND", 3)]
    #[case("AND ", 4)]
    #[case("hello AND", 9)]
    #[case("hello AND ", 10)]
    #[case("hello OR", 8)]
    #[case("hello OR ", 9)]
    #[case("NOT", 3)]
    #[case("NOT ", 4)]
    #[case("hello &&", 8)]
    #[case("hello && ", 9)]
    #[case("hello ||", 8)]
    #[case("!", 1)]
    #[case("(NOT", 4)]
    fn test_after_operator(#[case] query: &str, #[case] cursor_pos: usize) {
        assert!(matches!(
            QueryAnalyzer::analyze(query, cursor_pos),
            CompletionContext::AfterOperator
        ));
    }

    // ==================== InGroup Context ====================
    #[rstest]
    #[case("(", 1, 1)]
    #[case("((", 2, 2)]
    #[case("(((", 3, 3)]
    #[case("(( ) (", 6, 2)] // two open, one closed, one open = 2
    fn test_in_group(
        #[case] query: &str,
        #[case] cursor_pos: usize,
        #[case] expected_depth: usize,
    ) {
        match QueryAnalyzer::analyze(query, cursor_pos) {
            CompletionContext::InGroup { depth } => {
                assert_eq!(depth, expected_depth);
            }
            other => panic!(
                "Expected InGroup with depth {}, got {:?}",
                expected_depth, other
            ),
        }
    }

    // ==================== Cursor Position Edge Cases ====================
    #[test]
    fn test_cursor_beyond_query_length() {
        // Cursor position beyond query length should be clamped
        match QueryAnalyzer::analyze("hello", 100) {
            CompletionContext::PartialFieldOrTerm { text, .. } => {
                assert_eq!(text, "hello");
            }
            other => panic!("Expected PartialFieldOrTerm, got {:?}", other),
        }
    }

    #[test]
    fn test_cursor_mid_word() {
        // Cursor in middle of "hello" at position 3
        match QueryAnalyzer::analyze("hello world", 3) {
            CompletionContext::PartialFieldOrTerm { text, start_pos } => {
                assert_eq!(text, "hel");
                assert_eq!(start_pos, 0);
            }
            other => panic!("Expected PartialFieldOrTerm, got {:?}", other),
        }
    }

    #[test]
    fn test_cursor_mid_field_value() {
        // Cursor in middle of field value
        match QueryAnalyzer::analyze("root:/etc/bin", 9) {
            CompletionContext::FieldValue {
                field,
                value,
                value_start,
            } => {
                assert_eq!(field, "root");
                assert_eq!(value, "/etc");
                assert_eq!(value_start, 5);
            }
            other => panic!("Expected FieldValue, got {:?}", other),
        }
    }

    // ==================== Complex Query Tests ====================
    #[test]
    fn test_complex_query_after_group() {
        // "(a OR b) AND |" - cursor after AND
        assert!(matches!(
            QueryAnalyzer::analyze("(a OR b) AND ", 13),
            CompletionContext::AfterOperator
        ));
    }

    #[test]
    fn test_complex_query_in_nested_group() {
        // "((a AND |" - typing inside nested group
        match QueryAnalyzer::analyze("((a AND ", 8) {
            CompletionContext::AfterOperator => {}
            other => panic!("Expected AfterOperator, got {:?}", other),
        }
    }

    #[test]
    fn test_complex_query_field_in_group() {
        // "(root:/etc|" - field:value inside group
        match QueryAnalyzer::analyze("(root:/etc", 10) {
            CompletionContext::FieldValue { field, value, .. } => {
                assert_eq!(field, "root");
                assert_eq!(value, "/etc");
            }
            other => panic!("Expected FieldValue, got {:?}", other),
        }
    }

    // ==================== Quote Handling Edge Cases ====================
    #[test]
    fn test_escaped_quote_closed() {
        // Properly closed string with escaped quote inside
        let query = r#""hello \"world\"""#;
        assert!(
            matches!(
                QueryAnalyzer::analyze(query, query.len()),
                CompletionContext::InQuotedString // ends without space, inside completed quote
            ) | matches!(
                QueryAnalyzer::analyze(query, query.len()),
                CompletionContext::AfterTerm
            )
        );
    }

    #[test]
    fn test_completed_quoted_string_with_space() {
        let query = r#""hello" "#;
        assert!(matches!(
            QueryAnalyzer::analyze(query, query.len()),
            CompletionContext::AfterTerm
        ));
    }

    // ==================== has_unclosed_quote Tests ====================
    #[rstest]
    #[case("", false)]
    #[case("hello", false)]
    #[case(r#""hello""#, false)]
    #[case(r#""hello"#, true)]
    #[case(r#"""#, true)]
    #[case(r#""hello\" world""#, false)] // escaped quote inside
    #[case(r#""hello\""#, true)] // escaped quote at end, still unclosed
    #[case(r#""a" "b""#, false)] // two complete strings
    #[case(r#""a" "b"#, true)] // second string unclosed
    fn test_has_unclosed_quote(#[case] input: &str, #[case] expected: bool) {
        assert_eq!(QueryAnalyzer::has_unclosed_quote(input), expected);
    }
}
