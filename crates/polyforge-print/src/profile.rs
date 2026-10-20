//! Printer profiles.
//!
//! A `PrinterProfile` describes the physical machine: build volume, nozzle,
//! firmware dialect, scripted start/end G-code and a default `SliceSettings`.
//! Built-in profiles mirror the popular machines shipped by ksengine.

use crate::types::{GCodeFlavor, SliceSettings};
use serde::{Deserialize, Serialize};

/// Physical description of a 3D printer plus its default slicing settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PrinterProfile {
    pub id: String,
    pub name: String,
    pub vendor: String,
    pub model: String,
    pub variant: String,

    // Build volume (mm)
    pub build_volume_x: f64,
    pub build_volume_y: f64,
    pub build_volume_z: f64,
    pub circular_bed: bool,
    pub bed_diameter: f64,

    // Extruder
    pub nozzle_diameter: f64,
    pub max_extruders: u32,

    pub gcode_flavor: GCodeFlavor,

    // Capabilities
    pub has_heated_bed: bool,
    pub has_enclosure: bool,
    pub has_auto_bed_leveling: bool,

    // Motion limits
    pub max_speed_x: f64,
    pub max_speed_y: f64,
    pub max_speed_z: f64,
    pub max_accel_x: f64,
    pub max_accel_y: f64,

    // Temperature limits
    pub max_hotend_temp: i32,
    pub max_bed_temp: i32,

    pub bed_surface: String,

    // Scripted G-code ({nozzle_temp}, {bed_temp}, {bed_max_x}, {bed_max_y})
    pub start_gcode: String,
    pub end_gcode: String,

    pub description: String,
    pub website: String,

    /// Settings applied when this printer becomes active.
    pub default_settings: SliceSettings,
}

impl Default for PrinterProfile {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: "Generic".into(),
            vendor: String::new(),
            model: String::new(),
            variant: String::new(),

            build_volume_x: 250.0,
            build_volume_y: 210.0,
            build_volume_z: 220.0,
            circular_bed: false,
            bed_diameter: 0.0,

            nozzle_diameter: 0.4,
            max_extruders: 1,

            gcode_flavor: GCodeFlavor::Marlin,

            has_heated_bed: true,
            has_enclosure: false,
            has_auto_bed_leveling: true,

            max_speed_x: 200.0,
            max_speed_y: 200.0,
            max_speed_z: 20.0,
            max_accel_x: 1000.0,
            max_accel_y: 1000.0,

            max_hotend_temp: 300,
            max_bed_temp: 120,

            bed_surface: "PEI".into(),

            start_gcode: String::new(),
            end_gcode: String::new(),

            description: String::new(),
            website: String::new(),

            default_settings: SliceSettings::default(),
        }
    }
}

impl PrinterProfile {
    /// True if the build volume and motion limits are self-consistent.
    pub fn validate(&self) -> Result<(), String> {
        if self.id.is_empty() {
            return Err("printer id must not be empty".into());
        }
        if self.build_volume_x <= 0.0 || self.build_volume_y <= 0.0 || self.build_volume_z <= 0.0 {
            return Err("build volume must be positive".into());
        }
        if self.nozzle_diameter <= 0.0 {
            return Err("nozzle diameter must be > 0".into());
        }
        if self.max_hotend_temp <= 0 {
            return Err("max_hotend_temp must be > 0".into());
        }
        Ok(())
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
    }

    /// Display name of this printer's firmware dialect.
    pub fn gcode_flavor_label(&self) -> &'static str {
        self.gcode_flavor.label()
    }    pub fn from_json(value: &serde_json::Value) -> Result<Self, serde_json::Error> {
        serde_json::from_value(value.clone())
    }

    /// All built-in profile ids, in display order.
    pub fn builtin_profile_ids() -> &'static [&'static str] {
        &[
            "generic_marlin",
            "generic_klipper",
            "prusa_mk4",
            "prusa_mini",
            "bambu_x1c",
            "bambu_a1",
            "creality_ender3v2",
            "creality_k1",
            "voron_2_4",
        ]
    }

    /// Instantiate every built-in profile.
    pub fn builtin_profiles() -> Vec<PrinterProfile> {
        Self::builtin_profile_ids()
            .iter()
            .filter_map(|id| Self::create_builtin(id))
            .collect()
    }

    /// Build a single built-in profile by id, or `None` if unknown.
    pub fn create_builtin(id: &str) -> Option<PrinterProfile> {
        let p = match id {
            "generic_marlin" => generic_marlin(),
            "generic_klipper" => generic_klipper(),
            "prusa_mk4" => prusa_mk4(),
            "prusa_mini" => prusa_mini(),
            "bambu_x1c" => bambu_x1c(),
            "bambu_a1" => bambu_a1(),
            "creality_ender3v2" => creality_ender3v2(),
            "creality_k1" => creality_k1(),
            "voron_2_4" => voron_2_4(),
            _ => return None,
        };
        Some(p)
    }
}

/// A registry of available printers with an active selection.
#[derive(Debug, Clone, Default)]
pub struct PrinterRegistry {
    profiles: Vec<PrinterProfile>,
    active: Option<usize>,
}

impl PrinterRegistry {
    /// Create a registry pre-populated with the built-in printers.
    pub fn with_builtins() -> Self {
        let profiles = PrinterProfile::builtin_profiles();
        let active = if profiles.is_empty() { None } else { Some(0) };
        Self { profiles, active }
    }

    pub fn profiles(&self) -> &[PrinterProfile] {
        &self.profiles
    }

    pub fn get(&self, id: &str) -> Option<&PrinterProfile> {
        self.profiles.iter().find(|p| p.id == id)
    }

    pub fn active(&self) -> Option<&PrinterProfile> {
        self.active.and_then(|i| self.profiles.get(i))
    }

    pub fn set_active(&mut self, id: &str) -> bool {
        if let Some(i) = self.profiles.iter().position(|p| p.id == id) {
            self.active = Some(i);
            true
        } else {
            false
        }
    }

    /// Add a custom profile. Rejects duplicates and invalid profiles.
    pub fn add(&mut self, profile: PrinterProfile) -> Result<(), String> {
        if self.get(&profile.id).is_some() {
            return Err(format!("printer '{}' already exists", profile.id));
        }
        profile.validate()?;
        self.profiles.push(profile);
        if self.active.is_none() {
            self.active = Some(self.profiles.len() - 1);
        }
        Ok(())
    }

    pub fn remove(&mut self, id: &str) -> bool {
        if self.active().map(|p| p.id == id).unwrap_or(false) {
            return false; // never remove the active printer
        }
        let before = self.profiles.len();
        self.profiles.retain(|p| p.id != id);
        if self.profiles.len() != before {
            self.active = self
                .active
                .map(|i| i.min(self.profiles.len().saturating_sub(1)));
            true
        } else {
            false
        }
    }
}

// ============================================================================
// Built-in profile definitions
// ============================================================================

fn base(id: &str, name: &str, vendor: &str, model: &str) -> PrinterProfile {
    PrinterProfile {
        id: id.into(),
        name: name.into(),
        vendor: vendor.into(),
        model: model.into(),
        default_settings: SliceSettings {
            printer_profile_id: id.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn generic_marlin() -> PrinterProfile {
    let mut p = base(
        "generic_marlin",
        "Generic Marlin (0.4 mm)",
        "Generic",
        "Marlin",
    );
    p.description = "Generic i3-style Marlin printer, 220x220x250 mm.".into();
    p
}

fn generic_klipper() -> PrinterProfile {
    let mut p = base(
        "generic_klipper",
        "Generic Klipper (0.4 mm)",
        "Generic",
        "Klipper",
    );
    p.gcode_flavor = GCodeFlavor::Klipper;
    p.max_speed_x = 300.0;
    p.max_speed_y = 300.0;
    p.max_accel_x = 3000.0;
    p.max_accel_y = 3000.0;
    p.default_settings.gcode_flavor = GCodeFlavor::Klipper;
    p.default_settings.print_speed = 80.0;
    p.description = "Generic Klipper printer with input shaping.".into();
    p
}

fn prusa_mk4() -> PrinterProfile {
    let mut p = base("prusa_mk4", "Prusa MK4", "Prusa Research", "MK4");
    p.build_volume_x = 250.0;
    p.build_volume_y = 210.0;
    p.build_volume_z = 220.0;
    p.gcode_flavor = GCodeFlavor::Prusa;
    p.max_hotend_temp = 300;
    p.max_bed_temp = 120;
    p.bed_surface = "Textured PEI".into();
    p.website = "https://www.prusa3d.com".into();
    p.default_settings.gcode_flavor = GCodeFlavor::Prusa;
    p.default_settings.nozzle_temp = 215;
    p.default_settings.bed_temp = 60;
    p.description = "Original Prusa MK4 (Nextruder, 0.4 mm).".into();
    p
}

fn prusa_mini() -> PrinterProfile {
    let mut p = base("prusa_mini", "Prusa MINI+", "Prusa Research", "MINI+");
    p.build_volume_x = 180.0;
    p.build_volume_y = 180.0;
    p.build_volume_z = 180.0;
    p.gcode_flavor = GCodeFlavor::Prusa;
    p.default_settings.gcode_flavor = GCodeFlavor::Prusa;
    p.description = "Original Prusa MINI+ (0.4 mm).".into();
    p
}

fn bambu_x1c() -> PrinterProfile {
    let mut p = base("bambu_x1c", "Bambu Lab X1 Carbon", "Bambu Lab", "X1C");
    p.build_volume_x = 256.0;
    p.build_volume_y = 256.0;
    p.build_volume_z = 256.0;
    p.gcode_flavor = GCodeFlavor::Bambu;
    p.has_enclosure = true;
    p.max_hotend_temp = 300;
    p.max_bed_temp = 120;
    p.max_speed_x = 500.0;
    p.max_speed_y = 500.0;
    p.max_accel_x = 10000.0;
    p.max_accel_y = 10000.0;
    p.default_settings.gcode_flavor = GCodeFlavor::Bambu;
    p.default_settings.print_speed = 120.0;
    p.description = "Bambu Lab X1 Carbon with AMS.".into();
    p
}

fn bambu_a1() -> PrinterProfile {
    let mut p = base("bambu_a1", "Bambu Lab A1", "Bambu Lab", "A1");
    p.build_volume_x = 256.0;
    p.build_volume_y = 256.0;
    p.build_volume_z = 256.0;
    p.gcode_flavor = GCodeFlavor::Bambu;
    p.max_speed_x = 500.0;
    p.max_speed_y = 500.0;
    p.max_accel_x = 10000.0;
    p.max_accel_y = 10000.0;
    p.default_settings.gcode_flavor = GCodeFlavor::Bambu;
    p.default_settings.print_speed = 120.0;
    p.description = "Bambu Lab A1 bed slinger.".into();
    p
}

fn creality_ender3v2() -> PrinterProfile {
    let mut p = base(
        "creality_ender3v2",
        "Creality Ender-3 V2",
        "Creality",
        "Ender-3 V2",
    );
    p.build_volume_x = 220.0;
    p.build_volume_y = 220.0;
    p.build_volume_z = 250.0;
    p.gcode_flavor = GCodeFlavor::Marlin;
    p.max_speed_x = 200.0;
    p.max_speed_y = 200.0;
    p.max_accel_x = 500.0;
    p.max_accel_y = 500.0;
    p.bed_surface = "Glass".into();
    p.default_settings.print_speed = 45.0;
    p.description = "Creality Ender-3 V2 (Marlin, glass bed).".into();
    p
}

fn creality_k1() -> PrinterProfile {
    let mut p = base("creality_k1", "Creality K1", "Creality", "K1");
    p.build_volume_x = 220.0;
    p.build_volume_y = 220.0;
    p.build_volume_z = 250.0;
    p.gcode_flavor = GCodeFlavor::Klipper;
    p.has_enclosure = true;
    p.max_speed_x = 500.0;
    p.max_speed_y = 500.0;
    p.max_accel_x = 8000.0;
    p.max_accel_y = 8000.0;
    p.default_settings.gcode_flavor = GCodeFlavor::Klipper;
    p.default_settings.print_speed = 100.0;
    p.description = "Creality K1 CoreXY (Klipper).".into();
    p
}

fn voron_2_4() -> PrinterProfile {
    let mut p = base("voron_2_4", "Voron 2.4 (350)", "Voron Design", "2.4");
    p.build_volume_x = 350.0;
    p.build_volume_y = 350.0;
    p.build_volume_z = 330.0;
    p.gcode_flavor = GCodeFlavor::Klipper;
    p.has_enclosure = true;
    p.max_speed_x = 500.0;
    p.max_speed_y = 500.0;
    p.max_accel_x = 8000.0;
    p.max_accel_y = 8000.0;
    p.default_settings.gcode_flavor = GCodeFlavor::Klipper;
    p.default_settings.print_speed = 120.0;
    p.description = "Voron 2.4 350 mm CoreXY (Klipper).".into();
    p
}
