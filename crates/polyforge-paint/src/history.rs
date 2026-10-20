//! Undo/redo history for paint documents.
//!
//! Snapshots are whole-document clones taken once per operation (before the
//! operation runs), which keeps the brush engine branch-free while the memory
//! cost stays bounded by [`PaintHistory::LIMIT`].

use crate::types::{PaintChannel, PaintDocument};

/// One recorded state of a channel document.
#[derive(Debug, Clone, PartialEq)]
pub struct PaintSnapshot {
    pub channel: PaintChannel,
    pub document: PaintDocument,
}

/// Bounded undo/redo stacks of document snapshots.
#[derive(Debug, Clone)]
pub struct PaintHistory {
    undo: Vec<PaintSnapshot>,
    redo: Vec<PaintSnapshot>,
}

impl Default for PaintHistory {
    fn default() -> Self {
        Self::new()
    }
}

impl PaintHistory {
    /// Maximum number of undo steps kept.
    pub const LIMIT: usize = 16;

    pub fn new() -> Self {
        Self {
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }

    /// Record the state of a document *before* the next mutation.
    ///
    /// Recording the same state twice in a row is a no-op, which keeps
    /// continuous edits (slider drags) from flooding the stack.
    pub fn record(&mut self, channel: PaintChannel, document: &PaintDocument) {
        let identical = self
            .undo
            .last()
            .is_some_and(|last| last.channel == channel && last.document == *document);
        if !identical {
            self.undo.push(PaintSnapshot {
                channel,
                document: document.clone(),
            });
            if self.undo.len() > Self::LIMIT {
                self.undo.remove(0);
            }
        }
        self.redo.clear();
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn undo_len(&self) -> usize {
        self.undo.len()
    }

    /// Pop the previous state, pushing `current` onto the redo stack.
    pub fn undo(&mut self, current_channel: PaintChannel, current: PaintDocument) -> Option<PaintSnapshot> {
        let snapshot = self.undo.pop()?;
        self.redo.push(PaintSnapshot {
            channel: current_channel,
            document: current,
        });
        Some(snapshot)
    }

    /// Re-apply a state previously undone, pushing `current` back to undo.
    pub fn redo(&mut self, current_channel: PaintChannel, current: PaintDocument) -> Option<PaintSnapshot> {
        let snapshot = self.redo.pop()?;
        self.undo.push(PaintSnapshot {
            channel: current_channel,
            document: current,
        });
        Some(snapshot)
    }

    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }
}
