pub enum ServerStatus {
    Online(ServerWorkingStatus),
    Offline
}

pub enum ServerWorkingStatus {
    Indexing(IndexingProgressInfo),
    Searching
}

pub struct IndexingProgressInfo {
    total: usize,
    current: usize
}
