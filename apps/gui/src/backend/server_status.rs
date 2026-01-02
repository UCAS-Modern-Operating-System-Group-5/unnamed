#[derive(Clone)]
pub enum ServerStatus {
    Online(ServerWorkingStatus),
    Offline
}

#[derive(Clone)]
pub enum ServerWorkingStatus {
    Indexing(IndexingProgressInfo),
    Searching
}

#[derive(Clone)]
pub struct IndexingProgressInfo {
    total: usize,
    current: usize
}

impl std::fmt::Display for ServerStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServerStatus::Online(ws) => {
                match ws {
                    ServerWorkingStatus::Indexing(info) => {
                        write!(f, "Indexing ({}/{})", info.current, info.total)
                    },
                    ServerWorkingStatus::Searching => {
                        write!(f, "Searching")
                    }
                }
            },
            ServerStatus::Offline => {
                write!(f, "Offline")
            }
        }
    }
}
