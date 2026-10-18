//! Scene hierarchy and collections.

use bevy::prelude::*;

/// Named collection of entities (like Blender collections).
#[derive(Component, Reflect, Debug, Clone)]
#[reflect(Component)]
pub struct Collection {
    pub name: String,
    pub visible: bool,
}

impl Default for Collection {
    fn default() -> Self {
        Self {
            name: "Collection".into(),
            visible: true,
        }
    }
}

/// Marks membership in a collection entity.
#[derive(Component, Reflect, Debug, Clone, Copy)]
#[reflect(Component)]
pub struct InCollection(pub Entity);

/// Parent object in a simple hierarchy (transform relative optional).
#[derive(Component, Reflect, Debug, Clone, Copy)]
#[reflect(Component)]
pub struct ObjectParent(pub Entity);

/// Display name for outliner.
#[derive(Component, Reflect, Debug, Clone)]
#[reflect(Component)]
pub struct ObjectName(pub String);

impl Default for ObjectName {
    fn default() -> Self {
        Self("Object".into())
    }
}

/// Resource tracking root collection entity.
#[derive(Resource, Debug, Clone)]
pub struct SceneRoot {
    pub root_collection: Option<Entity>,
}

impl Default for SceneRoot {
    fn default() -> Self {
        Self {
            root_collection: None,
        }
    }
}

/// Hide/show renderable children of collections that are not visible.
#[derive(Component, Reflect, Debug, Clone, Copy, Default)]
#[reflect(Component)]
pub struct HiddenByCollection;


/// Local transform relative to [`ObjectParent`] (translation only in Phase 1).
#[derive(Component, Reflect, Debug, Clone, Copy)]
#[reflect(Component)]
pub struct ObjectLocal {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Default for ObjectLocal {
    fn default() -> Self {
        Self {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }
}
