use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use strum::{EnumCount, EnumIter};

#[derive(
    Debug,
    Default,
    PartialEq,
    Serialize,
    Deserialize,
    strum::Display,
    EnumIter,
    EnumCount,
    Clone,
)]
pub enum SortMode {
    FilePath,
    /// Sort by AI relevance score
    #[default]
    Score,
    ModifiedTime,
    AccessedTime,
    CreatedTime,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortDirection {
    #[default]
    Ascending,
    Descending,
}

impl SortDirection {
    pub fn toggle(&self) -> Self {
        match self {
            SortDirection::Ascending => SortDirection::Descending,
            SortDirection::Descending => SortDirection::Ascending,
        }
    }

    /// Apply direction to an ordering
    pub fn apply(&self, ordering: Ordering) -> Ordering {
        match self {
            SortDirection::Ascending => ordering,
            SortDirection::Descending => ordering.reverse(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SortConfig {
    pub mode: SortMode,
    pub direction: SortDirection,
}

impl Default for SortConfig {
    fn default() -> Self {
        let mode = SortMode::default();
        let direction = Self::default_direction_for(&mode);
        Self { mode, direction }
    }
}

impl SortConfig {
    pub fn new(mode: SortMode, direction: SortDirection) -> Self {
        Self { mode, direction }
    }

    /// If same mode, toggle direction; otherwise switch mode with default direction
    pub fn toggle_or_set(&mut self, mode: SortMode) {
        if self.mode == mode {
            self.direction = self.direction.toggle();
        } else {
            self.direction = Self::default_direction_for(&mode);
            self.mode = mode;
        }
    }

    /// Sensible default directions per mode
    fn default_direction_for(mode: &SortMode) -> SortDirection {
        match mode {
            // Newest/highest first for time and relevance
            SortMode::AccessedTime
            | SortMode::CreatedTime
            | SortMode::ModifiedTime
            | SortMode::Score => SortDirection::Descending,
            // A-Z for text-based
            SortMode::FilePath => SortDirection::Ascending,
        }
    }
}
