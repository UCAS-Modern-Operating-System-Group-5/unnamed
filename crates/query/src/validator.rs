// use crate::parser::{ParsedQuery, ParsedTerm};
// use std::ops::Range;
// use regex::Regex;

// // pub type ValidationResult = Result<>;

// pub enum Query {
//     Term(Term),
//     And(Vec<Query>),
//     Or(Vec<Query>),
//     Not(Box<Query>),
// }

// pub enum Term {
//     /// Root directory
//     Root(String),
//     /// Regular Expression
//     Regex(Regex),
//     /// Include glob pattern (e.g. *.png)
//     Glob(String),
//     Include(String),
//     /// Exclude glob pattern (e.g. *.png)
//     Exclude(String),
//     /// Access Time
//     ATime(Range<Option<u64>>),
//     /// Modified Time,
//     ModifiedTime(Range<Option<u64>>),
//     /// File Size
//     Size(Range<Option<u64>>),
// }

// pub fn validate_query(query: &ParsedQuery) {
//     match query {
//         ParsedQuery::Term(term) => {
//             validate_term(term);
//         },
//         ParsedQuery::And(items) => {},
//         ParsedQuery::Or(items) => todo!(),
//         ParsedQuery::Not(query) => todo!(),
//     }
// }

// // pub fn validate_term(parsed_term: &ParsedTerm) -> Result<Term, String> {
// //     // Ok
// // }
