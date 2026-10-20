//! Bridge between PolyForge and ArtCraft upstream apps (VectorCraft, PhotoCraft).
//!
//! Upstream trees live under `vendor/` and are refreshed by
//! `scripts/sync_artcraft.sh`. This crate does **not** recompile those apps into
//! the Bevy process (they are full editors). It:
//! - records integration metadata from `vendor/pins.toml` (when present)
//! - can spawn the external editors when built/installed beside PolyForge
//! - exposes format affinities (SVG ↔ VectorCraft, raster/PSD ↔ PhotoCraft)

use bevy::prelude::*;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct PolyForgeBridgePlugin;

impl Plugin for PolyForgeBridgePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ArtCraftIntegration>()
            .add_systems(Startup, load_artcraft_pins);
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct UpstreamPin {
    pub repo: String,
    pub homepage: String,
    pub license: String,
    pub pin_ref: String,
    pub pin_sha: String,
    pub last_checked: String,
    pub role: String,
    pub polyforge_mode: String,
}

#[derive(Resource, Debug, Clone, Default)]
pub struct ArtCraftIntegration {
    pub vectorcraft: UpstreamPin,
    pub photocraft: UpstreamPin,
    pub vendor_root: PathBuf,
    pub vectorcraft_available: bool,
    pub photocraft_available: bool,
}

fn default_vendor_root() -> PathBuf {
    // Walk up from CARGO_MANIFEST_DIR-style guesses at runtime
    PathBuf::from("vendor")
}

fn load_artcraft_pins(mut integ: ResMut<ArtCraftIntegration>) {
    integ.vendor_root = default_vendor_root();
    let pins_path = integ.vendor_root.join("pins.toml");
    if let Ok(text) = std::fs::read_to_string(&pins_path) {
        if let Some(vc) = parse_section(&text, "vectorcraft") {
            integ.vectorcraft = vc;
        }
        if let Some(pc) = parse_section(&text, "photocraft") {
            integ.photocraft = pc;
        }
    }
    integ.vectorcraft_available = integ.vendor_root.join("vectorcraft").join("Cargo.toml").is_file();
    integ.photocraft_available = integ.vendor_root.join("photocraft").join("Cargo.toml").is_file();
    bevy::log::info!(
        "ArtCraft bridge: VectorCraft {} ({}), PhotoCraft {} ({})",
        if integ.vectorcraft_available { "vendor OK" } else { "not vendored" },
        short(&integ.vectorcraft.pin_sha),
        if integ.photocraft_available { "vendor OK" } else { "not vendored" },
        short(&integ.photocraft.pin_sha),
    );
}

fn short(s: &str) -> &str {
    if s.len() >= 12 { &s[..12] } else if s.is_empty() { "unpinned" } else { s }
}

fn parse_section(text: &str, section: &str) -> Option<UpstreamPin> {
    let marker = format!("[{section}]");
    let start = text.find(&marker)? + marker.len();
    let rest = &text[start..];
    let end = rest.find("\n[").unwrap_or(rest.len());
    let block = &rest[..end];
    let get = |key: &str| -> String {
        for line in block.lines() {
            let line = line.trim();
            if let Some(rest) = line.strip_prefix(key) {
                let rest = rest.trim().trim_start_matches('=').trim();
                return rest.trim_matches('"').to_string();
            }
        }
        String::new()
    };
    Some(UpstreamPin {
        repo: get("repo"),
        homepage: get("homepage"),
        license: get("license"),
        pin_ref: get("pin_ref"),
        pin_sha: get("pin_sha"),
        last_checked: get("last_checked"),
        role: get("role"),
        polyforge_mode: get("polyforge_mode"),
    })
}

/// Try to run VectorCraft from vendor or PATH.
pub fn launch_vectorcraft(vendor_root: &Path) -> Result<(), String> {
    let local = vendor_root.join("vectorcraft");
    if local.join("Cargo.toml").is_file() {
        Command::new("cargo")
            .args(["run", "--release", "-p", "vectorcraft"])
            .current_dir(&local)
            .spawn()
            .map_err(|e| e.to_string())?;
        return Ok(());
    }
    Command::new("vectorcraft")
        .spawn()
        .map_err(|_| String::from("VectorCraft not found in vendor/ or PATH. Run ./scripts/sync_artcraft.sh"))?;
    Ok(())
}

/// Try to run PhotoCraft from vendor or PATH.
pub fn launch_photocraft(vendor_root: &Path) -> Result<(), String> {
    let local = vendor_root.join("photocraft");
    if local.join("Cargo.toml").is_file() {
        Command::new("cargo")
            .args(["run", "--release", "-p", "photocraft"])
            .current_dir(&local)
            .spawn()
            .map_err(|e| e.to_string())?;
        return Ok(());
    }
    Command::new("photocraft")
        .spawn()
        .map_err(|_| String::from("PhotoCraft not found in vendor/ or PATH. Run ./scripts/sync_artcraft.sh"))?;
    Ok(())
}

/// Formats VectorCraft is expected to exchange with PolyForge VectorMode.
pub const VECTOR_EXCHANGE: &[&str] = &["svg", "svgz", "pdf"];

/// Formats PhotoCraft is expected to exchange with PolyForge materials / textures.
pub const PHOTO_EXCHANGE: &[&str] = &["png", "jpeg", "jpg", "webp", "psd", "tiff"];
