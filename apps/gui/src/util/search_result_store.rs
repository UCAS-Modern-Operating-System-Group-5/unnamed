//! Search Result Store is used to store search results.
//! It allows user to do push results into the store with O(1) time complexity
//! It allows to change sort mode
//! The sort for every sort mode switch is O(n log n) once.
//! Note we assume every time there is 30+ items to arrive.
//! Using binary insert for single item is O(log n) (find) + O(n) (vec insert)
//! It's better to directly uses sort_by in this case (Rust uses TimSort, which is already
//! fast enough for mostly sorted list). 

use crate::util::{SortConfig, SortMode};
use rpc::search::SearchHit;
use std::cmp::Ordering;

pub struct SearchResultStore {
    /// All results in arrival order (append-only)
    results: Vec<SearchHit>,
    sorted_indices: Vec<usize>,
    
    sort_config: SortConfig,
    
    /// Whether sorted_indices needs rebuilding
    dirty: bool,
}



impl Default for SearchResultStore {
    fn default() -> Self {
        Self {
            results: Vec::new(),
            sorted_indices: Vec::new(),
            sort_config: SortConfig::default(),
            dirty: false,
        }
    }
}


impl SearchResultStore {
    pub fn with_sort_config(sort_config: SortConfig) -> Self {
        Self {
            results: Vec::new(),
            sorted_indices: Vec::new(),
            sort_config,
            dirty: false,
        }
    }
    
    pub fn extend(&mut self, hits: impl IntoIterator<Item = SearchHit>) {
        self.results.extend(hits);
        self.dirty = true;
    }
    
    pub fn clear(&mut self) {
        self.results.clear();
        self.sorted_indices.clear();
        self.dirty = false;
    }
    
    pub fn sort_config(&self) -> &SortConfig {
        &self.sort_config
    }
    

    pub fn set_sort_config(&mut self, config: SortConfig) {
        if self.sort_config != config {
            self.sort_config = config;
            self.dirty = true;
        }
    }
    
    /// Toggle direction for current mode
    pub fn toggle_direction(&mut self) {
        self.sort_config.direction = self.sort_config.direction.toggle();
        self.dirty = true;
    }
    
    /// Toggle direction if same mode, otherwise switch to mode with default direction
    pub fn toggle_or_set_mode(&mut self, mode: SortMode) {
        let old_config = self.sort_config.clone();
        self.sort_config.toggle_or_set(mode);
        if self.sort_config != old_config {
            self.dirty = true;
        }
    }

    pub fn len(&self) -> usize {
        self.results.len()
    }
    
    pub fn is_empty(&self) -> bool {
        self.results.is_empty()
    }
    
    /// Get item at sorted position (for table display)
    pub fn get_sorted(&mut self, index: usize) -> Option<&SearchHit> {
        self.ensure_sorted();
        self.sorted_indices
            .get(index)
            .map(|&idx| &self.results[idx])
    }
    
    /// Get the original index of the item at sorted position
    pub fn get_original_index(&mut self, sorted_index: usize) -> Option<usize> {
        self.ensure_sorted();
        self.sorted_indices.get(sorted_index).copied()
    }
    
    /// Iterate over results in sorted order
    pub fn iter_sorted(&mut self) -> impl Iterator<Item = &SearchHit> {
        self.ensure_sorted();
        self.sorted_indices.iter().map(|&idx| &self.results[idx])
    }
    
    /// Collect sorted results as a Vec (useful for snapshots)
    pub fn sorted_results(&mut self) -> Vec<&SearchHit> {
        self.iter_sorted().collect()
    }
    
    // ===== Sorting internals =====
    
    fn ensure_sorted(&mut self) {
        if !self.dirty {
            return;
        }
        self.sorted_indices = (0..self.results.len()).collect();
        
        let results = &self.results;
        let config = &self.sort_config;
        self.sorted_indices.sort_by(|&a, &b| {
            Self::compare_hits(&results[a], &results[b], config)
        });
        self.dirty = false;
    }
    
    fn compare_hits(a: &SearchHit, b: &SearchHit, config: &SortConfig) -> Ordering {
        let base_ordering = match config.mode {
            SortMode::FilePath => {
                // 按文件名排序，而不是完整路径
                let a_name = a.file_path.file_name().map(|s| s.to_ascii_lowercase());
                let b_name = b.file_path.file_name().map(|s| s.to_ascii_lowercase());
                a_name.cmp(&b_name)
                    .then_with(|| a.file_path.cmp(&b.file_path)) // 文件名相同时按完整路径排序
            }
            SortMode::AccessedTime => a.access_time.cmp(&b.access_time),
            SortMode::CreatedTime => a.create_time.cmp(&b.create_time),
            SortMode::ModifiedTime => a.modified_time.cmp(&b.modified_time),
            SortMode::Score => {
                // Score 从高到低排序，None 排在最后
                match (a.score, b.score) {
                    (Some(sa), Some(sb)) => sa.partial_cmp(&sb).unwrap_or(Ordering::Equal),
                    (Some(_), None) => Ordering::Greater, // 有分数的排前面
                    (None, Some(_)) => Ordering::Less,
                    (None, None) => Ordering::Equal,
                }.then_with(|| a.file_path.cmp(&b.file_path))
            }
        };
        config.direction.apply(base_ordering)
    }
}
