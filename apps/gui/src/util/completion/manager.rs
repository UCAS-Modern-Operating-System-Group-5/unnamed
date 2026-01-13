use super::{
    CompletionItem, CompletionResponse, CompletionSessionId, CompletionSource,
    CompletionStream, PathCompleter, Replacement, ReplacementRange,
    prelude::*,
    query_analyzer::{CompletionContext, QueryAnalyzer},
    session::CompletionSession,
};
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::sync::Mutex;
use tokio_stream::{Stream, StreamExt};

pub struct CompletionManager {
    path_completer: PathCompleter,
    /// Current active session (if any)
    current_session: Arc<Mutex<Option<CompletionSession>>>,
}

impl CompletionManager {
    pub fn new(cwd: impl Into<PathBuf>) -> Self {
        Self {
            path_completer: PathCompleter::new(cwd),
            current_session: Arc::new(Mutex::new(None)),
        }
    }

    pub fn with_current_dir() -> std::io::Result<Self> {
        Ok(Self {
            path_completer: PathCompleter::with_current_dir()?,
            current_session: Arc::new(Mutex::new(None)),
        })
    }

    /// Start a new completion session, cancelling any existing one
    pub async fn start_session(
        &self,
        session_id: CompletionSessionId,
        query: &str,
        cursor_pos: usize,
    ) -> CompletionResponse {
        let context = QueryAnalyzer::analyze(query, cursor_pos);
        let stream = self.create_stream(context, query, cursor_pos).await;

        let mut session = CompletionSession::new(session_id, stream);
        let (items, has_more) = session.next_batch().await;
        let total_so_far = session.total_collected();

        // Store session for continuation
        *self.current_session.lock().await = Some(session);

        CompletionResponse::Batch {
            session_id,
            items,
            has_more,
            total_so_far,
        }
    }

    /// Continue fetching from current session
    pub async fn continue_session(
        &self,
        session_id: CompletionSessionId,
    ) -> CompletionResponse {
        let mut guard = self.current_session.lock().await;

        match &mut *guard {
            Some(session) if session.id() == session_id => {
                let (items, has_more) = session.next_batch().await;
                let total_so_far = session.total_collected();

                CompletionResponse::Batch {
                    session_id,
                    items,
                    has_more,
                    total_so_far,
                }
            }
            _ => CompletionResponse::Cancelled { session_id },
        }
    }

    /// Cancel a session
    pub async fn cancel_session(&self, session_id: CompletionSessionId) {
        let mut guard = self.current_session.lock().await;
        if let Some(session) = &*guard {
            if session.id() == session_id {
                *guard = None;
            }
        }
    }

    async fn create_stream(
        &self,
        context: CompletionContext,
        query: &str,
        cursor_pos: usize,
    ) -> CompletionStream {
        match context {
            CompletionContext::Empty => Box::pin(tokio_stream::empty()),

            CompletionContext::PartialFieldOrTerm { text, start_pos } => {
                let field_completions =
                    self.field_name_completions(&text, start_pos..cursor_pos);

                if text.starts_with('~') || text.contains('/') {
                    let path_stream = self.path_completer.complete(&text).await;
                    let path_stream =
                        WrapperRangeStream::new(path_stream, start_pos..cursor_pos);

                    let combined =
                        tokio_stream::iter(field_completions).chain(path_stream);
                    Box::pin(combined)
                } else {
                    Box::pin(tokio_stream::iter(field_completions))
                }
            }

            CompletionContext::FieldValue {
                field,
                value,
                value_start,
            } => {
                match field.as_str() {
                    "root" => {
                        let stream = self.path_completer.complete(&value).await;
                        Box::pin(WrapperRangeStream::new(stream, value_start..cursor_pos))
                    }
                    _ => Box::pin(tokio_stream::empty()),
                }
            }

            CompletionContext::AfterTerm
            | CompletionContext::AfterOperator
            | CompletionContext::InGroup { .. } => Box::pin(tokio_stream::iter(
                self.field_name_completions("", cursor_pos..cursor_pos),
            )),

            CompletionContext::InQuotedString => Box::pin(tokio_stream::empty()),
        }
    }

    fn field_name_completions(
        &self,
        partial: &str,
        range: ReplacementRange,
    ) -> Vec<CompletionItem> {
        const FIELDS: &[(&str, &str)] = &[
            ("r:", "regexp"),
            ("key:", "Keyword"),
            ("root:", "Search root directory"),
            ("in:", "Include (glob)"),
            ("ext:", "Exclude (glob)"),
            ("atime:", "Access time range"),
            ("ctime:", "Create time range"),
            ("mtime:", "Modified time range"),
            ("size:", "File size range"),
            ("num:", "Number of results"),
        ];

        let partial_lower = partial.to_lowercase();

        FIELDS
            .iter()
            .filter(|(name, _)| name.starts_with(&partial_lower))
            .map(|(name, desc)| CompletionItem {
                label: format!("{} - {}", name, desc),
                replacement: Replacement {
                    range: range.clone(),
                    text: name.to_string(),
                },
                source: CompletionSource::Keyword,
            })
            .collect()
    }
}

/// A stream that wrapped the original stream, changes its replacement range information
struct WrapperRangeStream {
    inner: CompletionStream,
    range: ReplacementRange,
}

impl WrapperRangeStream {
    fn new(inner: CompletionStream, range: ReplacementRange) -> Self {
        Self { inner, range }
    }
}

impl Stream for WrapperRangeStream {
    type Item = CompletionItem;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        match Pin::new(&mut self.inner).poll_next(cx) {
            Poll::Ready(Some(mut item)) => {
                item.replacement = Replacement {
                    range: self.range.clone(),
                    ..item.replacement
                };
                Poll::Ready(Some(item))
            }
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}
