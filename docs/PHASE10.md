# Phase 10 — Multi-mesh restore, CAD edit polish, stronger 2D union

## Delivered

### Scene load
| Feature | Detail |
|---------|--------|
| Multi-mesh restore | meshes[0] → active; rest spawned as EditableObjects |
| CAD/vector dedupe | Despawn before load |

### CadMode
| Feature | Detail |
|---------|--------|
| Delete last feature | Task panel |
| Kernel label | ActiveKernel.backend |

### Vector boolean
| Feature | Detail |
|---------|--------|
| Union sampling | Boundary + interior grid → hull |

## Still later
- OCCT real linkage
- Concave union outline
- Poly object dedupe on multi-mesh load
