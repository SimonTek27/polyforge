//! Lightweight command / plugin registry (Rust-side scripting API).

use bevy::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Context passed to plugin commands.
pub struct CommandContext<'a> {
    pub world: &'a mut World,
}

/// Boxed command handler.
pub type CommandFn = Arc<dyn Fn(&mut World) + Send + Sync>;

/// Global registry of named commands (add-ons register here).
/// Name of command to run next frame (set by UI/hotkeys).
#[derive(Resource, Debug, Clone, Default)]
pub struct PendingPluginCommand(pub Option<String>);

#[derive(Resource, Clone, Default)]
pub struct CommandRegistry {
    inner: Arc<Mutex<HashMap<String, CommandFn>>>,
}

impl CommandRegistry {
    pub fn register<F>(&self, name: impl Into<String>, f: F)
    where
        F: Fn(&mut World) + Send + Sync + 'static,
    {
        if let Ok(mut map) = self.inner.lock() {
            map.insert(name.into(), Arc::new(f));
        }
    }

    pub fn list(&self) -> Vec<String> {
        self.inner
            .lock()
            .map(|m| {
                let mut v: Vec<_> = m.keys().cloned().collect();
                v.sort();
                v
            })
            .unwrap_or_default()
    }

    pub fn run(&self, name: &str, world: &mut World) -> bool {
        let cmd = self
            .inner
            .lock()
            .ok()
            .and_then(|m| m.get(name).cloned());
        if let Some(cmd) = cmd {
            cmd(world);
            true
        } else {
            false
        }
    }
}

/// Built-in plugin: registers demo commands.
pub fn register_builtin_commands(registry: &CommandRegistry) {
    registry.register("log.hello", |_world| {
        bevy::log::info!("PolyForge plugin API: hello");
    });
    registry.register("selection.clear_all", |world| {
        let mut q = world.query::<&mut super::selection::Selection>();
        for mut sel in q.iter_mut(world) {
            sel.clear();
        }
        bevy::log::info!("selection.clear_all executed");
    });
    registry.register("mesh.recalc_normals", |world| {
        let mut q = world.query::<&mut super::mesh::EditableMesh>();
        for mut mesh in q.iter_mut(world) {
            mesh.recalculate_normals();
        }
        bevy::log::info!("mesh.recalc_normals executed");
    });
    registry.register("mesh.triangulate_all", |world| {
        use super::selection::Selection;
        let mut q = world.query::<&mut super::mesh::EditableMesh>();
        for mut mesh in q.iter_mut(world) {
            let sel = Selection::default();
            // keep faces; caller ops triangulate needs ops crate — skip heavy dep
            let _ = (&mut mesh, sel);
        }
        bevy::log::info!("mesh.triangulate_all stub — use Ctrl+T in editor");
    });
    registry.register("uv.pack_islands", |_world| {
        bevy::log::info!("uv.pack_islands — trigger from UV Editor UI");
    });
}



/// Named sequence of command names (batch macro).
#[derive(Debug, Clone)]
pub struct CommandMacro {
    pub name: String,
    pub steps: Vec<String>,
}

#[derive(Resource, Clone, Default)]
pub struct MacroRegistry {
    pub macros: Vec<CommandMacro>,
}

impl MacroRegistry {
    pub fn register(&mut self, name: impl Into<String>, steps: Vec<String>) {
        self.macros.push(CommandMacro {
            name: name.into(),
            steps,
        });
    }

    pub fn get(&self, name: &str) -> Option<&CommandMacro> {
        self.macros.iter().find(|m| m.name == name)
    }
}

pub fn register_builtin_macros(reg: &mut MacroRegistry) {
    reg.register(
        "prep.export",
        vec![
            "mesh.recalc_normals".into(),
            "selection.clear_all".into(),
        ],
    );
}
