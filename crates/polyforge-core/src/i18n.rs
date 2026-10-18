//! Internationalization (i18n) for PolyForge UI and messages.
//!
//! English is the default locale. Add more languages via [`Locale`] + catalogs.

use bevy::prelude::*;
use std::collections::HashMap;

/// Supported UI languages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Reflect)]
pub enum Locale {
    #[default]
    English,
}

impl Locale {
    pub fn code(self) -> &'static str {
        match self {
            Locale::English => "en",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Locale::English => "English",
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        match code.to_ascii_lowercase().as_str() {
            "en" | "en-us" | "en-gb" => Some(Locale::English),
            _ => None,
        }
    }
}

/// Message keys for all user-visible UI strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Msg {
    // App
    AppTitle,
    // Menu – File
    MenuFile,
    MenuNewScene,
    MenuOpen,
    MenuSave,
    MenuQuit,
    // Menu – Edit
    MenuEdit,
    MenuUndo,
    MenuRedo,
    // Menu – View
    MenuView,
    MenuResetCamera,
    // Menu – Help
    MenuHelp,
    MenuAbout,
    // Toolbar
    Tools,
    ToolSelect,
    ToolMove,
    ToolRotate,
    ToolScale,
    ToolExtrude,
    ToolInset,
    ToolBevel,
    ToolPrimitive,
    // Tooltips
    TipSelect,
    TipMove,
    TipRotate,
    TipScale,
    TipExtrude,
    TipInset,
    TipBevel,
    TipPrimitive,
    // Properties
    Properties,
    SelectionMode,
    ModeVertex,
    ModeEdge,
    ModeFace,
    ModeCell,
    ModeObject,
    GizmoMode,
    Snap,
    SnapValue,
    ToolLabel,
    GizmoLabel,
    EditKeys,
    HintModes,
    HintTransform,
    HintExtrudeInset,
    HintLoopBevel,
    HintMergeDup,
    HintSplitDelete,
    HintEdgeLoop,
    HintLinkedGrow,
    HintUndo,
    HintTriangulate,
    HintSymExport,
    Language,
}

/// Runtime translation table + active locale.
#[derive(Resource, Debug, Clone)]
pub struct I18n {
    pub locale: Locale,
    catalogs: HashMap<Locale, HashMap<Msg, &'static str>>,
}

impl Default for I18n {
    fn default() -> Self {
        Self::with_locale(Locale::English)
    }
}

impl I18n {
    pub fn with_locale(locale: Locale) -> Self {
        let mut catalogs = HashMap::new();
        catalogs.insert(Locale::English, english_catalog());
        Self { locale, catalogs }
    }

    /// Translate a message key. Falls back to English, then `"<missing>"`.
    pub fn t(&self, key: Msg) -> &'static str {
        if let Some(cat) = self.catalogs.get(&self.locale) {
            if let Some(s) = cat.get(&key) {
                return s;
            }
        }
        if let Some(cat) = self.catalogs.get(&Locale::English) {
            if let Some(s) = cat.get(&key) {
                return s;
            }
        }
        "<missing>"
    }

    /// Localized selection summary, e.g. `"Selected: 2V  0E  1F/C"`.
    pub fn selected_counts(&self, v: usize, e: usize, f: usize) -> String {
        match self.locale {
            Locale::English => format!("Selected: {v}V  {e}E  {f}F/C"),
        }
    }

    pub fn set_locale(&mut self, locale: Locale) {
        self.locale = locale;
    }

    /// Human-readable tool name for status lines.
    pub fn tool_name(&self, tool: crate::Tool) -> &'static str {
        use crate::Tool;
        match tool {
            Tool::Select => self.t(Msg::ToolSelect),
            Tool::Move => self.t(Msg::ToolMove),
            Tool::Rotate => self.t(Msg::ToolRotate),
            Tool::Scale => self.t(Msg::ToolScale),
            Tool::Extrude => self.t(Msg::ToolExtrude),
            Tool::Inset => self.t(Msg::ToolInset),
            Tool::Bevel => self.t(Msg::ToolBevel),
            Tool::CreatePrimitive => self.t(Msg::ToolPrimitive),
            Tool::Bridge => "Bridge",
            Tool::Knife => "Knife",
        }
    }
}

fn english_catalog() -> HashMap<Msg, &'static str> {
    use Msg::*;
    HashMap::from([
        (AppTitle, "PolyForge"),
        (MenuFile, "File"),
        (MenuNewScene, "New Scene"),
        (MenuOpen, "Open..."),
        (MenuSave, "Save"),
        (MenuQuit, "Quit"),
        (MenuEdit, "Edit"),
        (MenuUndo, "Undo"),
        (MenuRedo, "Redo"),
        (MenuView, "View"),
        (MenuResetCamera, "Reset Camera"),
        (MenuHelp, "Help"),
        (MenuAbout, "About PolyForge"),
        (Tools, "Tools"),
        (ToolSelect, "Select"),
        (ToolMove, "Move"),
        (ToolRotate, "Rotate"),
        (ToolScale, "Scale"),
        (ToolExtrude, "Extrude"),
        (ToolInset, "Inset"),
        (ToolBevel, "Bevel"),
        (ToolPrimitive, "Prim"),
        (TipSelect, "Select elements (1/2/3/4/5 for mode)"),
        (TipMove, "Move selection (G)"),
        (TipRotate, "Rotate selection (R)"),
        (TipScale, "Scale selection (S)"),
        (TipExtrude, "Extrude selected faces (E)"),
        (TipInset, "Inset selected faces (I)"),
        (TipBevel, "Bevel selected edges (Ctrl+B)"),
        (TipPrimitive, "Create a primitive mesh"),
        (Properties, "Properties"),
        (SelectionMode, "Selection Mode"),
        (ModeVertex, "V"),
        (ModeEdge, "E"),
        (ModeFace, "F"),
        (ModeCell, "C"),
        (ModeObject, "O"),
        (GizmoMode, "Gizmo Mode"),
        (Snap, "Snap"),
        (SnapValue, "Snap value"),
        (ToolLabel, "Tool"),
        (GizmoLabel, "Gizmo"),
        (EditKeys, "Edit keys"),
        (HintModes, "1-5 = V/E/F/Cell/Obj"),
        (HintTransform, "G/R/S Move/Rot/Scale"),
        (HintExtrudeInset, "E Extrude | I Inset | F Fill"),
        (HintLoopBevel, "Ctrl+R LoopCut | Ctrl+B Bevel"),
        (HintMergeDup, "M Merge | Shift+D Duplicate"),
        (HintSplitDelete, "Y SplitEdge | X Delete"),
        (HintEdgeLoop, "Alt+click Edge loop"),
        (HintLinkedGrow, "L linked | +/- grow/shrink"),
        (HintUndo, "Ctrl+Z / Ctrl+Y Undo Redo"),
        (HintTriangulate, "Ctrl+T Triangulate | U Subdivide"),
        (HintSymExport, "Ctrl+Shift+X Sym X | Ctrl+Shift+E OBJ"),
        (Language, "Language"),
    ])
}
