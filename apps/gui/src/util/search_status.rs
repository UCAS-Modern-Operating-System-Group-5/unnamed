use uuid::Uuid;
use rpc::search::{SearchStatus as RpcSearchStatus, SearchErrorKind};


#[derive(Default)]
pub enum SearchStatus {
    #[default]
    Idle,
    Working(WorkingSearchStatus),
    Failed(SearchErrorKind)
}

pub struct WorkingSearchStatus {
    pub session_id: Uuid,
    pub status: Option<RpcSearchStatus>,
}

impl WorkingSearchStatus {
    pub fn new(id: Uuid) -> Self {
        Self {
            session_id: id,
            status: None
        }
    }
}

