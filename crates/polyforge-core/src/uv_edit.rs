
//! UV editor interaction state.

use bevy::prelude::*;
use std::collections::HashSet;

#[derive(Resource, Debug, Clone, Default)]
pub struct UvEditorState {
    pub selected: HashSet<u32>,
    pub dragging: bool,
    pub last_cursor: Option<Vec2>,
}

impl UvEditorState {
    pub fn clear(&mut self) {
        self.selected.clear();
        self.dragging = false;
        self.last_cursor = None;
    }
}
