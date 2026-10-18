//! Undo / redo history for mesh edits.
//!
//! Stores full mesh + selection snapshots (simple and reliable for mid-poly).

use bevy::prelude::*;
use super::mesh::EditableMesh;
use super::selection::Selection;

/// Maximum undo levels kept in memory.
pub const MAX_HISTORY: usize = 64;

#[derive(Clone, Debug)]
pub struct HistorySnapshot {
    pub mesh: EditableMesh,
    pub selection: Selection,
}

/// Per-active-object undo/redo stacks.
#[derive(Resource, Debug, Default)]
pub struct History {
    undo: Vec<HistorySnapshot>,
    redo: Vec<HistorySnapshot>,
    /// When true, the next mesh change should not push history
    /// (used while applying undo/redo itself).
    pub suppress: bool,
}

impl History {
    pub fn push(&mut self, mesh: &EditableMesh, selection: &Selection) {
        if self.suppress {
            return;
        }
        self.undo.push(HistorySnapshot {
            mesh: mesh.clone(),
            selection: selection.clone(),
        });
        if self.undo.len() > MAX_HISTORY {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Pop undo → return snapshot to restore. Current state goes to redo.
    pub fn undo(
        &mut self,
        current_mesh: &EditableMesh,
        current_sel: &Selection,
    ) -> Option<HistorySnapshot> {
        let snap = self.undo.pop()?;
        self.redo.push(HistorySnapshot {
            mesh: current_mesh.clone(),
            selection: current_sel.clone(),
        });
        Some(snap)
    }

    pub fn redo(
        &mut self,
        current_mesh: &EditableMesh,
        current_sel: &Selection,
    ) -> Option<HistorySnapshot> {
        let snap = self.redo.pop()?;
        self.undo.push(HistorySnapshot {
            mesh: current_mesh.clone(),
            selection: current_sel.clone(),
        });
        Some(snap)
    }

    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }
}
