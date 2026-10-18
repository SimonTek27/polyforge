//! Selection state for vertices, edges and faces.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Current selection on the active mesh.
#[derive(Component, Reflect, Debug, Clone, Default, Serialize, Deserialize)]
#[reflect(Component)]
pub struct Selection {
    pub vertices: HashSet<u32>,
    pub edges: HashSet<(u32, u32)>, // ordered pair (min, max)
    pub faces: HashSet<u32>,
}

impl Selection {
    pub fn clear(&mut self) {
        self.vertices.clear();
        self.edges.clear();
        self.faces.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.vertices.is_empty() && self.edges.is_empty() && self.faces.is_empty()
    }

    // ── Vertices ──────────────────────────────────────────────

    pub fn select_vertex(&mut self, idx: u32) {
        self.vertices.insert(idx);
    }

    pub fn deselect_vertex(&mut self, idx: u32) {
        self.vertices.remove(&idx);
    }

    pub fn toggle_vertex(&mut self, idx: u32) {
        if !self.vertices.remove(&idx) {
            self.vertices.insert(idx);
        }
    }

    pub fn set_vertex(&mut self, idx: u32) {
        self.clear();
        self.vertices.insert(idx);
    }

    pub fn select_all_vertices(&mut self, vertex_count: usize) {
        self.clear();
        for i in 0..vertex_count {
            self.vertices.insert(i as u32);
        }
    }

    // ── Edges ─────────────────────────────────────────────────

    pub fn select_edge(&mut self, a: u32, b: u32) {
        let edge = if a < b { (a, b) } else { (b, a) };
        self.edges.insert(edge);
    }

    pub fn deselect_edge(&mut self, a: u32, b: u32) {
        let edge = if a < b { (a, b) } else { (b, a) };
        self.edges.remove(&edge);
    }

    pub fn toggle_edge(&mut self, a: u32, b: u32) {
        let edge = if a < b { (a, b) } else { (b, a) };
        if !self.edges.remove(&edge) {
            self.edges.insert(edge);
        }
    }

    pub fn set_edge(&mut self, a: u32, b: u32) {
        self.clear();
        self.select_edge(a, b);
    }

    // ── Faces ─────────────────────────────────────────────────

    pub fn select_face(&mut self, idx: u32) {
        self.faces.insert(idx);
    }

    pub fn deselect_face(&mut self, idx: u32) {
        self.faces.remove(&idx);
    }

    pub fn toggle_face(&mut self, idx: u32) {
        if !self.faces.remove(&idx) {
            self.faces.insert(idx);
        }
    }

    pub fn set_face(&mut self, idx: u32) {
        self.clear();
        self.faces.insert(idx);
    }

    pub fn select_all_faces(&mut self, face_count: usize) {
        self.clear();
        for i in 0..face_count {
            self.faces.insert(i as u32);
        }
    }

    /// When faces are selected, also collect all their vertices
    /// (useful for transform tools that operate on vertices).
    pub fn vertices_from_faces(&self, mesh_faces: &[super::mesh::Face]) -> HashSet<u32> {
        let mut verts = HashSet::new();
        for &fi in &self.faces {
            if let Some(face) = mesh_faces.get(fi as usize) {
                for &vi in &face.indices {
                    verts.insert(vi);
                }
            }
        }
        verts
    }

    /// When edges are selected, collect their vertices.
    pub fn vertices_from_edges(&self) -> HashSet<u32> {
        let mut verts = HashSet::new();
        for &(a, b) in &self.edges {
            verts.insert(a);
            verts.insert(b);
        }
        verts
    }

    /// Effective vertex set for transform operations:
    /// explicit vertices ∪ vertices of selected edges ∪ vertices of selected faces.
    pub fn effective_vertices(&self, mesh_faces: &[super::mesh::Face]) -> HashSet<u32> {
        let mut verts = self.vertices.clone();
        verts.extend(self.vertices_from_edges());
        verts.extend(self.vertices_from_faces(mesh_faces));
        verts
    }

    /// Replace or merge a set of vertices (used by area selection).
    pub fn set_vertices_from_iter<I: IntoIterator<Item = u32>>(&mut self, iter: I, additive: bool) {
        if !additive {
            self.clear();
        }
        for idx in iter {
            self.vertices.insert(idx);
        }
    }

    pub fn set_edges_from_iter<I: IntoIterator<Item = (u32, u32)>>(
        &mut self,
        iter: I,
        additive: bool,
    ) {
        if !additive {
            self.clear();
        }
        for (a, b) in iter {
            self.select_edge(a, b);
        }
    }

    pub fn set_faces_from_iter<I: IntoIterator<Item = u32>>(&mut self, iter: I, additive: bool) {
        if !additive {
            self.clear();
        }
        for idx in iter {
            self.faces.insert(idx);
        }
    }

    /// Invert selection for the active mode counts.
    pub fn invert_vertices(&mut self, vertex_count: usize) {
        let mut next = std::collections::HashSet::new();
        for i in 0..vertex_count {
            let i = i as u32;
            if !self.vertices.contains(&i) {
                next.insert(i);
            }
        }
        self.vertices = next;
        self.edges.clear();
        self.faces.clear();
    }

    pub fn invert_faces(&mut self, face_count: usize) {
        let mut next = std::collections::HashSet::new();
        for i in 0..face_count {
            let i = i as u32;
            if !self.faces.contains(&i) {
                next.insert(i);
            }
        }
        self.faces = next;
        self.vertices.clear();
        self.edges.clear();
    }

    pub fn invert_edges(&mut self, all_edges: &[(u32, u32)]) {
        let mut next = std::collections::HashSet::new();
        for &(a, b) in all_edges {
            let e = if a < b { (a, b) } else { (b, a) };
            if !self.edges.contains(&e) {
                next.insert(e);
            }
        }
        self.edges = next;
        self.vertices.clear();
        self.faces.clear();
    }
}
