mod path;
mod manager;
mod query_analyzer;
mod session;
mod state;

pub use path::PathCompleter;
pub use state::CompletionState;
pub use manager::CompletionManager;


pub mod prelude {
    pub use super::Completer;
}


use tokio_stream::Stream;
use std::boxed::Box;
use std::pin::Pin;


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionSource {
    FileSystem,
    Keyword,
}

type ReplacementRange = std::ops::Range<usize>;

#[derive(Debug, Clone)]
pub struct Replacement {
    /// The part of original text to be replaced
    pub range: ReplacementRange,
    /// The new text
    pub text: String
}

impl Replacement {
    pub fn text_only(text: String) -> Self {
        Self {
            range: (usize::MAX..usize::MAX),
            text
        }
    }
}

#[derive(Debug, Clone)]
pub struct CompletionItem {
    pub label: String,
    pub replacement: Replacement,
    #[allow(dead_code)]
    pub source: CompletionSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CompletionSessionId(pub u64);


#[derive(Debug)]
pub enum CompletionResponse {
    Batch {
        session_id: CompletionSessionId,
        items: Vec<CompletionItem>,
        has_more: bool,
        #[allow(dead_code)]
        total_so_far: usize,
    },
    
    Cancelled {
        session_id: CompletionSessionId,
    },
}

pub enum CompletionRequest {
    /// Start a new completion session
    StartCompletion {
        session_id: CompletionSessionId,
        query: String,
        cursor_pos: usize,
    },
    /// Request more completions from current session
    ContinueCompletion {
        session_id: CompletionSessionId,
    },
    /// Cancel current completion session
    CancelCompletion {
        session_id: CompletionSessionId,
    }
}



// If you are interested in why we use `Pin` here, this is a good material:
// https://rust-lang.github.io/async-book/part-reference/pinning.html#pinning-and-async-programming
pub type CompletionStream = Pin<Box<dyn Stream<Item=CompletionItem> + Send>>;

/// Specific completer that only complete prefix string within their knowledge.
/// That's to say, you should first extract the path prefix from the query string
/// and then send
pub trait Completer {
    async fn complete(&self, prefix: &str) -> CompletionStream;
}


