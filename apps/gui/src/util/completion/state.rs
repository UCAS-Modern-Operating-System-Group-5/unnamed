//! UI State for Streaming Completions

use super::{CompletionItem, CompletionSessionId};

#[derive(Default)]
pub struct CompletionState {
    /// All collected items so far
    pub items: Vec<CompletionItem>,
    /// Selected index
    pub selected: Option<usize>,
    pub session_id: Option<CompletionSessionId>,
    session_counter: u64,
    /// More items available
    pub has_more: bool,
    /// Loading more items
    pub loading: bool,
    /// The session ID that should replace items (if any)
    pending_replace_session: Option<CompletionSessionId>,
}

impl CompletionState {
    pub fn new_session_id(&mut self) -> CompletionSessionId {
        self.session_counter += 1;
        CompletionSessionId(self.session_counter)
    }

    #[allow(dead_code)]
    pub fn start_session(&mut self, id: CompletionSessionId) {
        self.items.clear();
        self.selected = None;
        self.session_id = Some(id);
        self.has_more = true;
        self.loading = true;
        self.pending_replace_session = None;
    }

    /// Start a new session but keep showing old items until first non-empty batch arrives.
    /// This prevents the popup from blinking during typing.
    pub fn start_session_preserve_items(&mut self, session_id: CompletionSessionId) {
        self.session_id = Some(session_id);
        self.loading = true;
        self.has_more = false;
        self.pending_replace_session = Some(session_id);
    }

    pub fn receive_batch(
        &mut self,
        session_id: CompletionSessionId,
        items: Vec<CompletionItem>,
        has_more: bool,
    ) {
        if self.session_id != Some(session_id) {
            return;
        }

        let should_replace = self.pending_replace_session == Some(session_id);

        if should_replace {
            if !items.is_empty() {
                self.items = items;
                self.selected = Some(0);
                self.pending_replace_session = None;
            } else if !has_more {
                self.items.clear();
                self.selected = None;
                self.pending_replace_session = None;
            }
            // If empty but has_more, keep waiting
        } else {
            // Normal append mode for subsequent batches
            let was_empty = self.items.is_empty();
            self.items.extend(items);
            if was_empty && !self.items.is_empty() {
                self.selected = Some(0);
            }
        }

        self.has_more = has_more;
        self.loading = has_more;
    }

    pub fn cancel(&mut self, session_id: CompletionSessionId) {
        if self.session_id == Some(session_id) {
            self.loading = false;
            self.has_more = false;
            
            if self.pending_replace_session == Some(session_id) {
                self.pending_replace_session = None;
            }
        }
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.selected = None;
        self.session_id = None;
        self.has_more = false;
        self.loading = false;
        self.pending_replace_session = None;
    }

    pub fn select_next(&mut self) {
        if self.items.is_empty() {
            return;
        }
        self.selected = Some(match self.selected {
            Some(i) => (i + 1).min(self.items.len() - 1),
            None => 0,
        });
    }

    pub fn select_prev(&mut self) {
        if self.items.is_empty() {
            return;
        }
        self.selected = Some(match self.selected {
            Some(0) | None => 0,
            Some(i) => i - 1,
        });
    }
}
