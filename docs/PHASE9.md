# Phase 9 — Vector boolean 2D, multi-mesh scene, kernel stub

## Delivered

### VectorMode boolean 2D
| Op | Hotkey |
|----|--------|
| Union (convex hull) | Ctrl+Shift+U |
| Intersect (Sutherland–Hodgman) | Ctrl+Shift+J |
| Difference | Ctrl+Shift+D |

### Scene project
- meshes: Vec<NamedMeshData> — all EditableObject meshes saved
- Legacy mesh field kept for active mesh

### CadMode kernel
- SolidKernel trait + MeshKernel
- Feature flag cad_kernel reserved for OCCT
- Resource ActiveKernel

## Still later
- Real OCCT linkage
- Robust non-convex polygon boolean
- Restore multi-mesh on load as separate entities
