# Third-party licences

PolyForge is distributed under the terms in `LICENSE`. This file lists the
native and Rust dependencies whose licences require attribution.

## OpenCASCADE Technology (OCCT)

- Used by: `crates/polyforge-occt-sys` (C++ shim) and `crates/polyforge-cad`
  when built with the `cad_kernel` feature.
- Licence: GNU Lesser General Public License v2.1, with the OpenCASCADE
  exception (see `OCCT_LGPL_EXCEPTION.txt` in the OCCT distribution).
- Consequence: PolyForge links OCCT dynamically on the supported platforms so
  users can relink against a modified OCCT. Distributed binaries must ship the
  OCCT DLLs (or require a matching OCCT install) and reproduce the LGPL-2.1
  text plus the exception notice.
- Upstream: https://dev.opencascade.org/

## Rust dependencies

Crate licences are declared in each crate's `Cargo.toml` and mirrored in
`Cargo.lock`. The main runtime dependencies and their licence families:

| Dependency | Licence |
|------------|---------|
| Bevy | MIT OR Apache-2.0 |
| egui / bevy_egui | MIT OR Apache-2.0 |
| serde | MIT OR Apache-2.0 |
| glam | MIT OR Apache-2.0 |
| `cc` (build dependency) | MIT OR Apache-2.0 |

Generated SPDX data: run `cargo about generate` or `cargo deny check licenses`
when adding a new dependency that is not in the list above.
