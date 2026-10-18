# ADR: Solid kernel for CadMode

**Status:** Accepted for Phase 4–5 planning  
**Date:** 2026-10-17

## Context

CadMode needs exact solids (STEP, fillets, booleans). Phase 3–4 use mesh tessellation of parametric features, which is **not** FreeCAD parity.

## Options

| Option | Pros | Cons |
|--------|------|------|
| **A. OpenCASCADE (OCCT) via FFI / opencascade-rs** | Industry STEP, mature booleans | Heavy native deps, build complexity, license (LGPL) |
| **B. Pure Rust (e.g. truck / custom B-rep)** | Easy CI, pure cargo | Less STEP maturity, more engineering time |
| **C. Stay mesh-only** | Simple | Fails FreeCAD gap policy |

## Decision

1. **Short term (Phase 4–5):** keep mesh-parametric features + improve sketcher/UX.  
2. **Medium term:** prototype **Option A (OCCT)** behind a `cad_kernel` feature flag.  
3. **Long term:** re-evaluate pure Rust if OCCT packaging is too costly for GPL distribution.

## Consequences

- Do not claim B-rep parity until a kernel is integrated.  
- `mesh_union` / `mesh_fillet_approximate` remain viewport helpers only.  
- STEP IO blocked on kernel choice.
