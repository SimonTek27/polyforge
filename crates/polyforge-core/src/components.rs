//! ECS components for editable objects in the scene.

use bevy::prelude::*;

/// Marks an entity as an editable mesh object in the scene.
#[derive(Component, Debug, Clone, Default, Reflect)]
#[reflect(Component)]
pub struct EditableObject;

/// Marks the currently active mesh for editing tools.
#[derive(Component, Debug, Clone, Default, Reflect)]
#[reflect(Component)]
pub struct ActiveMesh;

/// Unique mesh identifier within a scene/document.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect, Default)]
#[reflect(Component)]
pub struct MeshId(pub u64);
