//! Multi-slot PBR material table for face assignment.

use bevy::prelude::*;

#[derive(Debug, Clone, Reflect)]
pub struct MaterialSlot {
    pub name: String,
    pub base_color: Vec4,
    pub metallic: f32,
    pub roughness: f32,
}

impl Default for MaterialSlot {
    fn default() -> Self {
        Self {
            name: "Material".into(),
            base_color: Vec4::new(0.7, 0.7, 0.7, 1.0),
            metallic: 0.0,
            roughness: 0.5,
        }
    }
}

/// Scene material table (index = face_materials entry).
#[derive(Resource, Debug, Clone)]
pub struct MaterialTable {
    pub slots: Vec<MaterialSlot>,
    pub active_slot: u32,
}

impl Default for MaterialTable {
    fn default() -> Self {
        Self {
            slots: vec![
                MaterialSlot {
                    name: "Default".into(),
                    base_color: Vec4::new(0.75, 0.75, 0.78, 1.0),
                    metallic: 0.05,
                    roughness: 0.55,
                },
                MaterialSlot {
                    name: "Accent".into(),
                    base_color: Vec4::new(0.2, 0.55, 0.9, 1.0),
                    metallic: 0.2,
                    roughness: 0.4,
                },
                MaterialSlot {
                    name: "Emit-ish".into(),
                    base_color: Vec4::new(0.9, 0.45, 0.15, 1.0),
                    metallic: 0.0,
                    roughness: 0.7,
                },
            ],
            active_slot: 0,
        }
    }
}

impl MaterialTable {
    pub fn ensure(&mut self, count: usize) {
        while self.slots.len() < count {
            let i = self.slots.len();
            self.slots.push(MaterialSlot {
                name: format!("Material.{i}"),
                ..Default::default()
            });
        }
    }
}
