//! Extra Bevy components used by the editor.

use bevy::prelude::*;

/// Marker for objects that can be edited in the viewport.
#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub struct EditableObject;

/// Marker for the currently active (focused) mesh.
#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub struct ActiveMesh;
