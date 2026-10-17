# Phase 11 — Poly dedupe on load, concave-ish union contour, vector clear

## Delivered

### Scene load
| Feature | Detail |
|---------|--------|
| Poly dedupe | Despawn non-active EditableObject (without CadBody) before multi-mesh restore |
| Multi-mesh restore | Active + spawned extras (Phase 10) |

### VectorMode
| Feature | Detail |
|---------|--------|
| Union contour | Grid boundary cells ordered by angle |
| Clear all paths | Ctrl+Shift+X |

### Still later
- OCCT
- True topological boolean 2D
- Selective path delete / node edit
