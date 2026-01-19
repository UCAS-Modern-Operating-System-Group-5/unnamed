pub mod lexer;
pub mod parser;
pub mod validator;

pub use lexer::{QueryLexer, Token};
pub use parser::{parse_query, parser, Span};
pub use validator::{
    FIELD_DEFINITIONS, Query, Term, ValidationError, ValidationErrorKind,
    ValidationResult, validate_query,
};

pub fn empty_span() -> Span {
    use chumsky::span::Span as _;
    Span::new((), 0..0)
}
