use super::{CompletionItem, CompletionSessionId, CompletionStream};
use tokio_stream::StreamExt;
use crate::constants;

pub struct CompletionSession {
    id: CompletionSessionId,
    stream: CompletionStream,
    collected: Vec<CompletionItem>,
    exhausted: bool,
    batch_size: usize,
}

impl CompletionSession {
    pub fn new(id: CompletionSessionId, stream: CompletionStream) -> Self {
        Self {
            id,
            stream,
            collected: Vec::new(),
            exhausted: false,
            batch_size: constants::COMPLETION_BATCH_SIZE,
        }
    }

    pub fn id(&self) -> CompletionSessionId {
        self.id
    }

    pub async fn next_batch(&mut self) -> (Vec<CompletionItem>, bool) {
        if self.exhausted {
            return (Vec::new(), false);
        }

        let mut batch = Vec::with_capacity(self.batch_size);

        for _ in 0..self.batch_size {
            match self.stream.next().await {
                Some(item) => {
                    self.collected.push(item.clone());
                    batch.push(item);
                }
                None => {
                    self.exhausted = true;
                    break;
                }
            }
        }

        let has_more = !self.exhausted;
        (batch, has_more)
    }

    pub fn total_collected(&self) -> usize {
        self.collected.len()
    }

    pub fn is_exhausted(&self) -> bool {
        self.exhausted
    }
}
