mod file_size;
mod time;

use crate::parser::{ParsedQuery, ParsedTerm, Span, Spanned};
pub use file_size::SizeRange;
use regex::Regex;
use std::fmt;
pub use time::TimeRange;

#[derive(Debug)]
pub enum Query {
    Term(Term),
    And(Vec<Query>),
    Or(Vec<Query>),
    Not(Box<Query>),
}

#[derive(Debug)]
pub enum Term {
    /// Root directory
    Root(String),
    /// A keyword that must be matched in file content
    KeyWord(String),
    /// Regular Expression
    Regex(Regex),
    /// Glob pattern (e.g. `*.pdf`, `!*.rs`)
    Glob(String),
    /// Access time range (Unix timestamp in seconds)
    AccessTime(TimeRange),
    /// Modified time range (Unix timestamp in seconds)
    ModifiedTime(TimeRange),
    /// Creation time range (Unix timestamp in seconds)
    CreatedTime(TimeRange),
    /// File size range (in bytes)
    Size(SizeRange),
}

#[derive(Debug, Clone)]
pub struct ValidationError {
    pub span: Span,
    pub kind: ValidationErrorKind,
}

impl ValidationError {
    pub fn new(span: Span, kind: ValidationErrorKind) -> Self {
        Self { span, kind }
    }

    /// Get the byte range of the error in the original input
    pub fn range(&self) -> std::ops::Range<usize> {
        self.span.start..self.span.end
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} (at position {}..{})",
            self.kind, self.span.start, self.span.end
        )
    }
}

impl std::error::Error for ValidationError {}

#[derive(Debug, Clone, PartialEq)]
pub enum ValidationErrorKind {
    UnknownField { field: String },
    InvalidRegex { pattern: String, reason: String },
    InvalidGlob { pattern: String, reason: String },
    InvalidTimeSpec { value: String, reason: String },
    InvalidSizeSpec { value: String, reason: String },
    EmptyValue,
    InvalidRange { reason: String },
}

impl fmt::Display for ValidationErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationErrorKind::UnknownField { field } => {
                write!(f, "unknown field '{}'", field)
            }
            ValidationErrorKind::InvalidRegex { pattern, reason } => {
                write!(f, "invalid regex '{}': {}", pattern, reason)
            }
            ValidationErrorKind::InvalidGlob { pattern, reason } => {
                write!(f, "invalid glob '{}': {}", pattern, reason)
            }
            ValidationErrorKind::InvalidTimeSpec { value, reason } => {
                write!(f, "invalid time '{}': {}", value, reason)
            }
            ValidationErrorKind::InvalidSizeSpec { value, reason } => {
                write!(f, "invalid size '{}': {}", value, reason)
            }
            ValidationErrorKind::EmptyValue => write!(f, "empty value"),
            ValidationErrorKind::InvalidRange { reason } => {
                write!(f, "invalid range: {}", reason)
            }
        }
    }
}

pub type ValidationResult<T> = Result<T, ValidationError>;

/// Validate a parsed query and convert it to a semantic query
pub fn validate_query(query: &Spanned<ParsedQuery>) -> ValidationResult<Query> {
    let (parsed, _span) = query;
    match parsed {
        ParsedQuery::Term(term) => validate_term(term).map(Query::Term),
        ParsedQuery::And(items) => {
            let validated: Result<Vec<_>, _> = items.iter().map(validate_query).collect();
            validated.map(Query::And)
        }
        ParsedQuery::Or(items) => {
            let validated: Result<Vec<_>, _> = items.iter().map(validate_query).collect();
            validated.map(Query::Or)
        }
        ParsedQuery::Not(inner) => validate_query(inner).map(|q| Query::Not(Box::new(q))),
    }
}

pub struct FieldDef {
    pub kind: FieldKind,
    pub aliases: &'static [&'static str],
    pub description: &'static str,
}

impl FieldDef {
    /// Find a field definition by any of its aliases
    pub fn find_by_alias(name: &str) -> Option<&'static FieldDef> {
        let name_lower = name.to_lowercase();
        FIELD_DEFINITIONS
            .iter()
            .find(|def| def.aliases.iter().any(|&a| a == name_lower))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldKind {
    Root,
    KeyWord,
    Regex,
    Glob,
    AccessTime,
    ModifiedTime,
    CreatedTime,
    Size,
}

impl FieldKind {
    /// Parse a value string into a Term based on the field kind
    pub fn parse_value(&self, value: String, span: Span) -> ValidationResult<Term> {
        match self {
            FieldKind::Root => Ok(Term::Root(value)),
            FieldKind::KeyWord => Ok(Term::KeyWord(value)),
            FieldKind::Regex => validate_regex(value, span).map(Term::Regex),
            FieldKind::Glob => Ok(Term::Glob(value)),
            FieldKind::AccessTime => {
                time::validate_time(value, span).map(Term::AccessTime)
            }
            FieldKind::ModifiedTime => {
                time::validate_time(value, span).map(Term::ModifiedTime)
            }
            FieldKind::CreatedTime => {
                time::validate_time(value, span).map(Term::CreatedTime)
            }
            FieldKind::Size => file_size::validate_size(value, span).map(Term::Size),
        }
    }
}

pub static FIELD_DEFINITIONS: &[FieldDef] = &[
    FieldDef {
        kind: FieldKind::Root,
        aliases: &["root", "path"],
        description: "Search root directory",
    },
    FieldDef {
        aliases: &["key", "keyword"],
        description: "Keyword match",
        kind: FieldKind::Regex,
    },
    FieldDef {
        aliases: &["r", "re", "regex", "regexp"],
        description: "Regular expression pattern",
        kind: FieldKind::Regex,
    },
    FieldDef {
        kind: FieldKind::Glob,
        aliases: &["glob", "name", "filename", "file"],
        description: "Glob/filename pattern",
    },
    FieldDef {
        kind: FieldKind::AccessTime,
        aliases: &["atime", "access", "accessed"],
        description: "Access time range",
    },
    FieldDef {
        kind: FieldKind::ModifiedTime,
        aliases: &["mtime", "mod", "modified"],
        description: "Modified time range",
    },
    FieldDef {
        kind: FieldKind::CreatedTime,
        aliases: &["ctime", "create", "created"],
        description: "Creation time range",
    },
    FieldDef {
        kind: FieldKind::Size,
        aliases: &["s", "size", "bytes"],
        description: "File size range",
    },
];

/// Validate a parsed term and convert it to a semantic term
fn validate_term(term: &ParsedTerm) -> ValidationResult<Term> {
    let (value, value_span) = &term.value;
    let value_string = value.to_string();

    if value_string.is_empty() {
        return Err(ValidationError::new(
            *value_span,
            ValidationErrorKind::EmptyValue,
        ));
    }

    match &term.field {
        None => Ok(Term::KeyWord(value_string.into())),
        Some((field, field_span)) => match FieldDef::find_by_alias(field) {
            Some(def) => def.kind.parse_value(value_string, *value_span),
            None => Err(ValidationError::new(
                *field_span,
                ValidationErrorKind::UnknownField {
                    field: field.clone(),
                },
            )),
        },
    }
}

fn validate_regex(pattern: String, span: Span) -> ValidationResult<Regex> {
    match Regex::new(&pattern) {
        Ok(re) => Ok(re),
        Err(e) => Err(ValidationError::new(
            span,
            ValidationErrorKind::InvalidRegex {
                pattern: pattern.into(),
                reason: e.to_string(),
            },
        )),
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::parser::parse_query;
    use rstest::rstest;

    // Helper function
    fn validate(input: &str) -> ValidationResult<Query> {
        let parsed = parse_query(input).expect("Parse failed");
        validate_query(&parsed)
    }

    // Helper to extract error kind
    fn validate_err(input: &str) -> ValidationErrorKind {
        validate(input).unwrap_err().kind
    }

    // ==================== Basic Term Tests ====================

    #[test]
    fn test_bare_keyword() {
        let query = validate("hello").unwrap();
        assert!(matches!(query, Query::Term(Term::KeyWord(k)) if k == "hello"));
    }

    #[test]
    fn test_quoted_keyword() {
        let query = validate(r#""hello world""#).unwrap();
        assert!(matches!(query, Query::Term(Term::KeyWord(k)) if k == "hello world"));
    }

    #[test]
    fn test_field_root() {
        let query = validate("root:/home/user").unwrap();
        assert!(matches!(query, Query::Term(Term::Root(p)) if p == "/home/user"));
    }

    // ==================== Field Alias Tests ====================

    #[rstest]
    #[case("root:/path")]
    #[case("path:/path")]
    fn test_root_aliases(#[case] input: &str) {
        let query = validate(input).unwrap();
        assert!(matches!(query, Query::Term(Term::Root(p)) if p == "/path"));
    }

    #[rstest]
    #[case("r:test.*")]
    #[case("re:test.*")]
    #[case("regex:test.*")]
    #[case("regexp:test.*")]
    fn test_regex_aliases(#[case] input: &str) {
        let query = validate(input).unwrap();
        assert!(matches!(query, Query::Term(Term::Regex(_))));
    }

    #[rstest]
    #[case("glob:*.rs")]
    #[case("name:*.rs")]
    #[case("filename:*.rs")]
    #[case("file:*.rs")]
    fn test_glob_aliases(#[case] input: &str) {
        let query = validate(input).unwrap();
        assert!(matches!(query, Query::Term(Term::Glob(g)) if g == "*.rs"));
    }

    #[rstest]
    #[case("atime:>1d")]
    #[case("access:>1d")]
    #[case("accessed:>1d")]
    fn test_access_time_aliases(#[case] input: &str) {
        let query = validate(input).unwrap();
        assert!(matches!(query, Query::Term(Term::AccessTime(_))));
    }

    #[rstest]
    #[case("mtime:>1d")]
    #[case("mod:>1d")]
    #[case("modified:>1d")]
    fn test_modified_time_aliases(#[case] input: &str) {
        let query = validate(input).unwrap();
        assert!(matches!(query, Query::Term(Term::ModifiedTime(_))));
    }

    #[rstest]
    #[case("ctime:>1d")]
    #[case("create:>1d")]
    #[case("created:>1d")]
    fn test_created_time_aliases(#[case] input: &str) {
        let query = validate(input).unwrap();
        assert!(matches!(query, Query::Term(Term::CreatedTime(_))));
    }

    #[rstest]
    #[case("s:>1MB")]
    #[case("size:>1MB")]
    #[case("bytes:>1MB")]
    fn test_size_aliases(#[case] input: &str) {
        let query = validate(input).unwrap();
        assert!(matches!(query, Query::Term(Term::Size(_))));
    }

    // ==================== Case Insensitivity Tests ====================

    #[rstest]
    #[case("ROOT:/path")]
    #[case("Root:/path")]
    #[case("rOoT:/path")]
    fn test_field_case_insensitive(#[case] input: &str) {
        let query = validate(input).unwrap();
        assert!(matches!(query, Query::Term(Term::Root(_))));
    }

    // ==================== Regex Validation Tests ====================

    #[test]
    fn test_valid_regex() {
        let query = validate(r"regex:^test\d+\.rs$").unwrap();
        if let Query::Term(Term::Regex(re)) = query {
            assert!(re.is_match("test123.rs"));
            assert!(!re.is_match("test.rs"));
        } else {
            panic!("Expected regex term");
        }
    }

    #[test]
    fn test_invalid_regex() {
        let err = validate_err(r"regex:[invalid");
        assert!(matches!(err, ValidationErrorKind::InvalidRegex { .. }));
    }

    #[test]
    fn test_invalid_regex_unclosed_group() {
        let err = validate_err(r#"regex:"(unclosed""#);
        assert!(matches!(err, ValidationErrorKind::InvalidRegex { .. }));
    }

    // ==================== Compound Query Tests ====================

    #[test]
    fn test_and_query() {
        let query = validate("foo AND bar").unwrap();
        if let Query::And(items) = query {
            assert_eq!(items.len(), 2);
            assert!(matches!(&items[0], Query::Term(Term::KeyWord(k)) if k == "foo"));
            assert!(matches!(&items[1], Query::Term(Term::KeyWord(k)) if k == "bar"));
        } else {
            panic!("Expected AND query");
        }
    }

    #[test]
    fn test_or_query() {
        let query = validate("foo OR bar").unwrap();
        if let Query::Or(items) = query {
            assert_eq!(items.len(), 2);
            assert!(matches!(&items[0], Query::Term(Term::KeyWord(k)) if k == "foo"));
            assert!(matches!(&items[1], Query::Term(Term::KeyWord(k)) if k == "bar"));
        } else {
            panic!("Expected OR query");
        }
    }

    #[test]
    fn test_not_query() {
        let query = validate("NOT foo").unwrap();
        if let Query::Not(inner) = query {
            assert!(matches!(*inner, Query::Term(Term::KeyWord(k)) if k == "foo"));
        } else {
            panic!("Expected NOT query");
        }
    }

    #[test]
    fn test_nested_query() {
        let query = validate("(foo AND bar) OR baz").unwrap();
        if let Query::Or(items) = query {
            assert_eq!(items.len(), 2);
            assert!(matches!(&items[0], Query::And(_)));
            assert!(matches!(&items[1], Query::Term(Term::KeyWord(k)) if k == "baz"));
        } else {
            panic!("Expected OR query with nested AND");
        }
    }

    #[test]
    fn test_complex_query() {
        let query =
            validate("root:/home AND (name:*.rs OR name:*.toml) AND NOT size:>10MB")
                .unwrap();
        if let Query::And(items) = query {
            assert_eq!(items.len(), 3);
            assert!(matches!(&items[0], Query::Term(Term::Root(_))));
            assert!(matches!(&items[1], Query::Or(_)));
            assert!(matches!(&items[2], Query::Not(_)));
        } else {
            panic!("Expected complex AND query");
        }
    }

    // ==================== Error Tests ====================

    #[test]
    fn test_unknown_field() {
        let err = validate_err("unknown:value");
        assert!(
            matches!(err, ValidationErrorKind::UnknownField { field } if field == "unknown")
        );
    }

    #[test]
    fn test_empty_value() {
        // This depends on parser behavior - adjust if needed
        let result = validate(r#"root:"""#);
        if let Err(e) = result {
            assert!(matches!(e.kind, ValidationErrorKind::EmptyValue));
        }
    }

    // ==================== Field Definition Tests ====================

    #[test]
    fn test_find_by_alias_exists() {
        let def = FieldDef::find_by_alias("regex").unwrap();
        assert_eq!(def.kind, FieldKind::Regex);
    }

    #[test]
    fn test_find_by_alias_not_exists() {
        let def = FieldDef::find_by_alias("nonexistent");
        assert!(def.is_none());
    }

    #[test]
    fn test_all_field_definitions_have_aliases() {
        for def in FIELD_DEFINITIONS {
            assert!(
                !def.aliases.is_empty(),
                "Field {:?} has no aliases",
                def.kind
            );
        }
    }

    // ==================== ValidationError Tests ====================

    #[test]
    fn test_validation_error_display() {
        let err = ValidationError::new(
            Span {
                start: 5,
                end: 10,
                context: (),
            },
            ValidationErrorKind::UnknownField {
                field: "foo".into(),
            },
        );
        let display = format!("{}", err);
        assert!(display.contains("unknown field 'foo'"));
        assert!(display.contains("5..10"));
    }

    #[test]
    fn test_validation_error_range() {
        let err = ValidationError::new(
            Span {
                start: 5,
                end: 10,
                context: (),
            },
            ValidationErrorKind::EmptyValue,
        );
        assert_eq!(err.range(), 5..10);
    }

    #[rstest]
    #[case(ValidationErrorKind::UnknownField { field: "x".into() }, "unknown field")]
    #[case(ValidationErrorKind::InvalidRegex { pattern: "[".into(), reason: "err".into() }, "invalid regex")]
    #[case(ValidationErrorKind::InvalidGlob { pattern: "**[".into(), reason: "err".into() }, "invalid glob")]
    #[case(ValidationErrorKind::InvalidTimeSpec { value: "bad".into(), reason: "err".into() }, "invalid time")]
    #[case(ValidationErrorKind::InvalidSizeSpec { value: "bad".into(), reason: "err".into() }, "invalid size")]
    #[case(ValidationErrorKind::EmptyValue, "empty value")]
    #[case(ValidationErrorKind::InvalidRange { reason: "err".into() }, "invalid range")]
    fn test_error_kind_display(
        #[case] kind: ValidationErrorKind,
        #[case] expected_substr: &str,
    ) {
        let display = format!("{}", kind);
        assert!(
            display.contains(expected_substr),
            "Expected '{}' to contain '{}'",
            display,
            expected_substr
        );
    }

    // ==================== TimeRange Tests ====================

    #[test]
    fn test_time_range_at_least() {
        let range = TimeRange::at_least(100);
        assert!(range.contains(100));
        assert!(range.contains(200));
        assert!(!range.contains(99));
    }

    #[test]
    fn test_time_range_at_most() {
        let range = TimeRange::at_most(100);
        assert!(range.contains(100));
        assert!(range.contains(50));
        assert!(!range.contains(101));
    }

    #[test]
    fn test_time_range_between() {
        let range = TimeRange::between(50, 100);
        assert!(range.contains(50));
        assert!(range.contains(75));
        assert!(range.contains(100));
        assert!(!range.contains(49));
        assert!(!range.contains(101));
    }

    #[test]
    fn test_time_range_unbounded() {
        let range = TimeRange {
            min: None,
            max: None,
        };
        assert!(range.contains(0));
        assert!(range.contains(u64::MAX));
    }

    // ==================== SizeRange Tests ====================

    #[test]
    fn test_size_range_at_least() {
        let range = SizeRange::at_least(1024);
        assert!(range.contains(1024));
        assert!(range.contains(2048));
        assert!(!range.contains(1023));
    }

    #[test]
    fn test_size_range_at_most() {
        let range = SizeRange::at_most(1024);
        assert!(range.contains(1024));
        assert!(range.contains(512));
        assert!(!range.contains(1025));
    }

    #[test]
    fn test_size_range_between() {
        let range = SizeRange::between(100, 200);
        assert!(range.contains(100));
        assert!(range.contains(150));
        assert!(range.contains(200));
        assert!(!range.contains(99));
        assert!(!range.contains(201));
    }

    #[test]
    fn test_size_range_exactly() {
        let range = SizeRange::exactly(1024);
        assert!(range.contains(1024));
        assert!(!range.contains(1023));
        assert!(!range.contains(1025));
    }

    // ==================== Integration Tests ====================

    #[test]
    fn test_realistic_query_source_search() {
        // Search for Rust files modified in the last week, larger than 1KB
        let query = validate("root:/home/dev AND name:*.rs AND mtime:<1w AND size:>1KB");
        assert!(query.is_ok());
    }

    #[test]
    fn test_realistic_query_log_search() {
        // Search for log files accessed recently with specific pattern
        let query = validate("glob:*.log AND atime:<1d AND regex:ERROR|WARN");
        assert!(query.is_ok());
    }

    #[test]
    fn test_realistic_query_cleanup() {
        // Find large old files for cleanup
        let query =
            validate("(size:>100MB AND mtime:>30d) OR (name:*.tmp AND mtime:>7d)");
        assert!(query.is_ok());
    }
}
