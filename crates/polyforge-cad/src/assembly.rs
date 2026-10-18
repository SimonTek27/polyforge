//! Simple assembly mates (Phase 6 MVP).

use bevy::prelude::*;

/// Mate type between two CAD bodies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect)]
pub enum MateKind {
    /// Snap child origin to parent origin.
    CoincidentOrigin,
    /// Keep offset along axis.
    OffsetAxis,
}

#[derive(Component, Reflect, Debug, Clone)]
#[reflect(Component)]
pub struct AssemblyMate {
    pub parent: Entity,
    pub kind: MateKind,
    pub offset: Vec3,
}

impl Default for AssemblyMate {
    fn default() -> Self {
        Self {
            parent: Entity::PLACEHOLDER,
            kind: MateKind::CoincidentOrigin,
            offset: Vec3::ZERO,
        }
    }
}

/// Apply mates each frame (translation only).
pub fn apply_assembly_mates(
    parents: Query<&Transform, Without<AssemblyMate>>,
    mut children: Query<(&AssemblyMate, &mut Transform)>,
) {
    for (mate, mut tf) in &mut children {
        let Ok(parent_tf) = parents.get(mate.parent) else {
            continue;
        };
        match mate.kind {
            MateKind::CoincidentOrigin => {
                tf.translation = parent_tf.translation + mate.offset;
            }
            MateKind::OffsetAxis => {
                tf.translation = parent_tf.translation + parent_tf.rotation * mate.offset;
            }
        }
    }
}


use std::collections::HashMap;

/// Compute depth of each mated entity (0 = root / no mate).
pub fn mate_depths(mates: &[(Entity, Entity)]) -> HashMap<Entity, u32> {
    // mates: (child, parent)
    let parent_of: HashMap<Entity, Entity> = mates.iter().copied().collect();
    let mut depths = HashMap::new();
    for (child, _) in mates {
        let mut d = 0u32;
        let mut cur = *child;
        let mut guard = 0;
        while let Some(&p) = parent_of.get(&cur) {
            d += 1;
            cur = p;
            guard += 1;
            if guard > 64 {
                break;
            }
        }
        depths.insert(*child, d);
    }
    depths
}

/// Root-first ordering for tree display.
pub fn sort_by_hierarchy(bodies: &[Entity], mates: &[(Entity, Entity)]) -> Vec<(Entity, u32)> {
    let depths = mate_depths(mates);
    let mut items: Vec<(Entity, u32)> = bodies
        .iter()
        .map(|e| (*e, depths.get(e).copied().unwrap_or(0)))
        .collect();
    items.sort_by_key(|(e, d)| (*d, e.to_bits()));
    items
}
