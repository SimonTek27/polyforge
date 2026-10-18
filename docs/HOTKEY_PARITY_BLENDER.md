# Hotkey parity vs Blender (Phase 1)

Blender defaults on the left; PolyForge on the right.

| Action | Blender | PolyForge | Status |
|--------|---------|-----------|--------|
| Vertex/Edge/Face mode | 1 / 2 / 3 | 1 / 2 / 3 | ✅ |
| Object mode | Tab (toggle) | Tab (Edit↔Object) | ✅ |
| Select all | A | A | ✅ |
| Box select | B | drag LMB | 🟡 |
| Circle select | C | C | ✅ |
| Lasso | Ctrl+RMB drag | Ctrl+L | 🟡 |
| Edge loop | Alt+click | Alt+click | ✅ |
| Edge ring | Ctrl+Alt+click | Ctrl+Alt+click | ✅ |
| Linked | L | L | ✅ |
| Invert | Ctrl+I | Ctrl+I | ✅ |
| Select similar | Shift+G | Shift+G | ✅ |
| Grab / Rotate / Scale | G R S | G R S | ✅ |
| Extrude | E | E | ✅ |
| Inset | I | I | ✅ |
| Bevel | Ctrl+B | Ctrl+B | ✅ |
| Loop cut | Ctrl+R | Ctrl+R | ✅ |
| Knife | K | K mid / Ctrl+K stroke | ✅ |
| Fill | F | F | ✅ |
| Merge | M menu | M | 🟡 |
| Delete | X | X | ✅ |
| Duplicate | Shift+D | Shift+D | ✅ |
| Separate | P | P | ✅ |
| Join | Ctrl+J | Ctrl+J | ✅ |
| Triangulate | Ctrl+T | Ctrl+T | ✅ |
| Subdivide | W menu / right-click | W / U | ✅ |
| Rip | V | V | ✅ |
| Rip fill | Alt+V | Alt+V / Shift+V | ✅ |
| Spin | Spin tool | Shift+W | 🟡 |
| Screw | Screw tool | Shift+Q | 🟡 |
| Proportional | O | O | ✅ |
| Snap cycle | Shift+Tab / menu | Ctrl+` | 🟡 |
| Undo / Redo | Ctrl+Z / Ctrl+Shift+Z | Ctrl+Z / Ctrl+Y | ✅ |
| Save | Ctrl+S | Ctrl+S | ✅ |
| Open | Ctrl+O | Ctrl+O | ✅ |
| Hide collection | H | Shift+H | 🟡 |
| Plugin list | — | F8 | extra |
| Plugin clear sel | — | F9 | extra |
| Plugin recalc nrm | — | F10 | extra |

## Parity pass notes

- Prefer Blender bindings when adding new operators.
- Document intentional differences (e.g. Tab cycles objects, not Edit/Object mode).
- Next pass: align Subdivide to W, Rip fill to Alt+V, Object mode Tab if Edit/Object toggle is introduced.

| Run macro prep.export | — | F7 | extra |
