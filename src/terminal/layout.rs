use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TerminalLayout {
    /// A single terminal leaf node (by index)
    Terminal(usize),
    /// A horizontal split (left-to-right)
    Horizontal {
        splits: Vec<TerminalLayout>,
        weights: Vec<f32>,
    },
    /// A vertical split (top-to-bottom)
    Vertical {
        splits: Vec<TerminalLayout>,
        weights: Vec<f32>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveDirection {
    Up,
    Down,
    Left,
    Right,
}

enum NavigationResult {
    FoundTarget,
    FoundNewFocus(usize),
    NotFound,
}

impl TerminalLayout {
    /// Get the first terminal ID in the layout (depth-first).
    pub fn first_leaf(&self) -> usize {
        match self {
            TerminalLayout::Terminal(id) => *id,
            TerminalLayout::Horizontal { splits, .. } | TerminalLayout::Vertical { splits, .. } => {
                splits.first().map(|s| s.first_leaf()).unwrap_or(0)
            }
        }
    }

    /// Get the last terminal ID in the layout (depth-first).
    pub fn last_leaf(&self) -> usize {
        match self {
            TerminalLayout::Terminal(id) => *id,
            TerminalLayout::Horizontal { splits, .. } | TerminalLayout::Vertical { splits, .. } => {
                splits.last().map(|s| s.last_leaf()).unwrap_or(0)
            }
        }
    }

    /// Navigate focus from `target_id` in `direction`.
    pub fn navigate(&self, target_id: usize, direction: MoveDirection) -> Option<usize> {
        match self.navigate_internal(target_id, direction) {
            NavigationResult::FoundNewFocus(id) => Some(id),
            _ => None,
        }
    }

    fn navigate_internal(&self, target_id: usize, direction: MoveDirection) -> NavigationResult {
        match self {
            TerminalLayout::Terminal(id) => {
                if *id == target_id {
                    NavigationResult::FoundTarget
                } else {
                    NavigationResult::NotFound
                }
            }
            TerminalLayout::Horizontal { splits, .. } => {
                for (i, split) in splits.iter().enumerate() {
                    match split.navigate_internal(target_id, direction) {
                        NavigationResult::FoundNewFocus(id) => {
                            return NavigationResult::FoundNewFocus(id)
                        }
                        NavigationResult::FoundTarget => {
                            // We found the target in this split. Can we move?
                            match direction {
                                MoveDirection::Left if i > 0 => {
                                    return NavigationResult::FoundNewFocus(
                                        splits[i - 1].last_leaf(),
                                    );
                                }
                                MoveDirection::Right if i < splits.len() - 1 => {
                                    return NavigationResult::FoundNewFocus(
                                        splits[i + 1].first_leaf(),
                                    );
                                }
                                _ => return NavigationResult::FoundTarget, // Hit boundary
                            }
                        }
                        NavigationResult::NotFound => continue,
                    }
                }
                NavigationResult::NotFound
            }
            TerminalLayout::Vertical { splits, .. } => {
                for (i, split) in splits.iter().enumerate() {
                    match split.navigate_internal(target_id, direction) {
                        NavigationResult::FoundNewFocus(id) => {
                            return NavigationResult::FoundNewFocus(id)
                        }
                        NavigationResult::FoundTarget => match direction {
                            MoveDirection::Up if i > 0 => {
                                return NavigationResult::FoundNewFocus(splits[i - 1].last_leaf());
                            }
                            MoveDirection::Down if i < splits.len() - 1 => {
                                return NavigationResult::FoundNewFocus(splits[i + 1].first_leaf());
                            }
                            _ => return NavigationResult::FoundTarget,
                        },
                        NavigationResult::NotFound => continue,
                    }
                }
                NavigationResult::NotFound
            }
        }
    }

    /// Split the pane containing `target_id` with a new terminal `new_id`.
    /// Returns true if successful.
    pub fn split(&mut self, target_id: usize, new_id: usize, direction: Direction) -> bool {
        match self {
            TerminalLayout::Terminal(id) => {
                if *id == target_id {
                    let old_leaf = TerminalLayout::Terminal(*id);
                    let new_leaf = TerminalLayout::Terminal(new_id);

                    *self = match direction {
                        Direction::Horizontal => TerminalLayout::Horizontal {
                            splits: vec![old_leaf, new_leaf],
                            weights: vec![0.5, 0.5],
                        },
                        Direction::Vertical => TerminalLayout::Vertical {
                            splits: vec![old_leaf, new_leaf],
                            weights: vec![0.5, 0.5],
                        },
                    };
                    return true;
                }
                false
            }
            TerminalLayout::Horizontal { splits, .. } => {
                for split in splits.iter_mut() {
                    if split.split(target_id, new_id, direction) {
                        return true;
                    }
                }
                false
            }
            TerminalLayout::Vertical { splits, .. } => {
                for split in splits.iter_mut() {
                    if split.split(target_id, new_id, direction) {
                        return true;
                    }
                }
                false
            }
        }
    }

    /// Split the pane containing `target_id` with an existing layout subtree.
    /// The `insert_before` flag determines if the new layout goes before (left/top) or after (right/bottom).
    /// Returns true if successful.
    pub fn split_with_layout(
        &mut self,
        target_id: usize,
        new_layout: TerminalLayout,
        direction: Direction,
        insert_before: bool,
    ) -> bool {
        match self {
            TerminalLayout::Terminal(id) => {
                if *id == target_id {
                    let old_leaf = TerminalLayout::Terminal(*id);
                    let (first, second) = if insert_before {
                        (new_layout, old_leaf)
                    } else {
                        (old_leaf, new_layout)
                    };

                    *self = match direction {
                        Direction::Horizontal => TerminalLayout::Horizontal {
                            splits: vec![first, second],
                            weights: vec![0.5, 0.5],
                        },
                        Direction::Vertical => TerminalLayout::Vertical {
                            splits: vec![first, second],
                            weights: vec![0.5, 0.5],
                        },
                    };
                    return true;
                }
                false
            }
            TerminalLayout::Horizontal { splits, .. } | TerminalLayout::Vertical { splits, .. } => {
                for split in splits.iter_mut() {
                    if split.split_with_layout(
                        target_id,
                        new_layout.clone(),
                        direction,
                        insert_before,
                    ) {
                        return true;
                    }
                }
                false
            }
        }
    }

    /// Collect all terminal IDs in the layout.
    pub fn collect_leaves(&self) -> Vec<usize> {
        match self {
            TerminalLayout::Terminal(id) => vec![*id],
            TerminalLayout::Horizontal { splits, .. } | TerminalLayout::Vertical { splits, .. } => {
                splits.iter().flat_map(|s| s.collect_leaves()).collect()
            }
        }
    }

    /// Remove a terminal ID from the layout.
    /// Returns true if the layout root itself should be removed (i.e. it was that terminal).
    pub fn remove_id(&mut self, target_id: usize) -> bool {
        match self {
            TerminalLayout::Terminal(id) => *id == target_id,
            TerminalLayout::Horizontal { splits, weights }
            | TerminalLayout::Vertical { splits, weights } => {
                let mut found_idx = None;
                for (i, split) in splits.iter_mut().enumerate() {
                    if split.remove_id(target_id) {
                        found_idx = Some(i);
                        break;
                    }
                }

                if let Some(idx) = found_idx {
                    splits.remove(idx);
                    if !weights.is_empty() {
                        weights.remove(idx.min(weights.len() - 1));
                        // Normalize weights
                        let total: f32 = weights.iter().sum();
                        if total > 0.0 {
                            for w in weights.iter_mut() {
                                *w /= total;
                            }
                        }
                    }

                    if splits.is_empty() {
                        return true;
                    }

                    if splits.len() == 1 {
                        // Collapse: replace self with the only remaining child
                        *self = splits.remove(0);
                    }
                }
                false // Root is not the terminal
            }
        }
    }

    /// Create a grid layout.
    /// Returns (layout, next_id).
    pub fn grid(rows: usize, cols: usize, start_id: usize) -> (Self, usize) {
        let mut next_id = start_id;
        let mut row_splits = Vec::new();

        for _ in 0..rows {
            let mut col_splits = Vec::new();
            for _ in 0..cols {
                col_splits.push(TerminalLayout::Terminal(next_id));
                next_id += 1;
            }

            if cols == 1 {
                row_splits.push(col_splits.remove(0));
            } else {
                row_splits.push(TerminalLayout::Horizontal {
                    splits: col_splits,
                    weights: vec![1.0 / cols as f32; cols],
                });
            }
        }

        if rows == 1 {
            (row_splits.remove(0), next_id)
        } else {
            (
                TerminalLayout::Vertical {
                    splits: row_splits,
                    weights: vec![1.0 / rows as f32; rows],
                },
                next_id,
            )
        }
    }
}
