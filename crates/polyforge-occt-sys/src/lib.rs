//! Raw C ABI shim + thin Rust bindings for OpenCASCADE (OCCT).
//!
//! The C++ shim (`src/shim.cpp`) keeps every OCCT type behind a `void*`
//! handle, so the rest of PolyForge never sees C++ templates or OCCT headers.
//!
//! Building the shim requires the `occt` feature and `OCCT_ROOT`; with the
//! feature disabled every function is a safe no-op returning `None` / `false`.

use std::ffi::c_void;
use std::path::Path;
#[cfg(feature = "occt")]
use std::ffi::{CStr, CString};

/// Whether this crate was built against OCCT.
pub const ENABLED: bool = cfg!(feature = "occt");

/// Owned opaque OCCT shape handle.
pub struct Shape {
    raw: *mut c_void,
}

impl Shape {
    pub fn as_raw(&self) -> *mut c_void {
        self.raw
    }

    pub fn is_null(&self) -> bool {
        self.raw.is_null()
    }
}

impl Drop for Shape {
    fn drop(&mut self) {
        #[cfg(feature = "occt")]
        {
            if !self.raw.is_null() {
                unsafe { ffi::pf_shape_free(self.raw) };
            }
        }
    }
}

#[cfg(feature = "occt")]
mod ffi {
    use std::ffi::{c_char, c_void};

    extern "C" {
        pub fn pf_occt_init() -> i32;
        pub fn pf_occt_version() -> *const c_char;

        pub fn pf_box(dx: f64, dy: f64, dz: f64) -> *mut c_void;
        pub fn pf_cylinder(radius: f64, height: f64) -> *mut c_void;
        pub fn pf_sphere(radius: f64) -> *mut c_void;
        pub fn pf_cone(r1: f64, r2: f64, height: f64) -> *mut c_void;
        pub fn pf_fuse(a: *mut c_void, b: *mut c_void) -> *mut c_void;
        pub fn pf_cut(a: *mut c_void, b: *mut c_void) -> *mut c_void;
        pub fn pf_fillet(shape: *mut c_void, radius: f64) -> *mut c_void;
        pub fn pf_shape_free(shape: *mut c_void);

        pub fn pf_triangulate(
            shape: *mut c_void,
            deflection: f64,
            out_positions: *mut *mut f64,
            out_triangle_count: *mut u32,
        ) -> i32;
        pub fn pf_positions_free(positions: *mut f64);

        pub fn pf_step_read(path: *const c_char, out: *mut *mut c_void) -> i32;
        pub fn pf_step_write(shape: *mut c_void, path: *const c_char) -> i32;
    }
}

fn wrap(raw: *mut c_void) -> Option<Shape> {
    if raw.is_null() {
        None
    } else {
        Some(Shape { raw })
    }
}

pub fn init() -> bool {
    #[cfg(feature = "occt")]
    {
        return unsafe { ffi::pf_occt_init() != 0 };
    }
    #[cfg(not(feature = "occt"))]
    false
}

pub fn version() -> Option<String> {
    #[cfg(feature = "occt")]
    {
        let ptr = unsafe { ffi::pf_occt_version() };
        if ptr.is_null() {
            return None;
        }
        let text = unsafe { CStr::from_ptr(ptr) };
        return Some(text.to_string_lossy().into_owned());
    }
    #[cfg(not(feature = "occt"))]
    None
}

pub fn make_box(dx: f64, dy: f64, dz: f64) -> Option<Shape> {
    #[cfg(feature = "occt")]
    {
        return wrap(unsafe { ffi::pf_box(dx, dy, dz) });
    }
    #[cfg(not(feature = "occt"))]
    {
        let _ = (dx, dy, dz);
        None
    }
}

pub fn make_cylinder(radius: f64, height: f64) -> Option<Shape> {
    #[cfg(feature = "occt")]
    {
        return wrap(unsafe { ffi::pf_cylinder(radius, height) });
    }
    #[cfg(not(feature = "occt"))]
    {
        let _ = (radius, height);
        None
    }
}

pub fn make_sphere(radius: f64) -> Option<Shape> {
    #[cfg(feature = "occt")]
    {
        return wrap(unsafe { ffi::pf_sphere(radius) });
    }
    #[cfg(not(feature = "occt"))]
    {
        let _ = radius;
        None
    }
}

pub fn make_cone(r1: f64, r2: f64, height: f64) -> Option<Shape> {
    #[cfg(feature = "occt")]
    {
        return wrap(unsafe { ffi::pf_cone(r1, r2, height) });
    }
    #[cfg(not(feature = "occt"))]
    {
        let _ = (r1, r2, height);
        None
    }
}

pub fn fuse(a: &Shape, b: &Shape) -> Option<Shape> {
    #[cfg(feature = "occt")]
    {
        return wrap(unsafe { ffi::pf_fuse(a.raw, b.raw) });
    }
    #[cfg(not(feature = "occt"))]
    {
        let _ = (a, b);
        None
    }
}

pub fn cut(a: &Shape, b: &Shape) -> Option<Shape> {
    #[cfg(feature = "occt")]
    {
        return wrap(unsafe { ffi::pf_cut(a.raw, b.raw) });
    }
    #[cfg(not(feature = "occt"))]
    {
        let _ = (a, b);
        None
    }
}

pub fn fillet(shape: &Shape, radius: f64) -> Option<Shape> {
    #[cfg(feature = "occt")]
    {
        return wrap(unsafe { ffi::pf_fillet(shape.raw, radius) });
    }
    #[cfg(not(feature = "occt"))]
    {
        let _ = (shape, radius);
        None
    }
}

/// Triangle-soup tessellation: vertices are duplicated per triangle.
pub fn triangulate(shape: &Shape, deflection: f64) -> Option<(Vec<[f32; 3]>, Vec<[u32; 3]>)> {
    #[cfg(feature = "occt")]
    {
        let mut positions: *mut f64 = std::ptr::null_mut();
        let mut triangles: u32 = 0;
        let ok = unsafe {
            ffi::pf_triangulate(shape.raw, deflection, &mut positions, &mut triangles)
        };
        if ok == 0 || positions.is_null() || triangles == 0 {
            if !positions.is_null() {
                unsafe { ffi::pf_positions_free(positions) };
            }
            return None;
        }

        let count = triangles as usize * 9;
        let slice = unsafe { std::slice::from_raw_parts(positions, count) };
        let mut vertices = Vec::with_capacity(triangles as usize * 3);
        for chunk in slice.chunks_exact(3) {
            vertices.push([chunk[0] as f32, chunk[1] as f32, chunk[2] as f32]);
        }
        unsafe { ffi::pf_positions_free(positions) };

        let faces = (0..triangles)
            .map(|i| [i * 3, i * 3 + 1, i * 3 + 2])
            .collect();
        return Some((vertices, faces));
    }
    #[cfg(not(feature = "occt"))]
    {
        let _ = (shape, deflection);
        None
    }
}

pub fn step_read(path: impl AsRef<Path>) -> Option<Shape> {
    #[cfg(feature = "occt")]
    {
        let path = CString::new(path.as_ref().to_string_lossy().as_bytes()).ok()?;
        let mut raw: *mut c_void = std::ptr::null_mut();
        let ok = unsafe { ffi::pf_step_read(path.as_ptr(), &mut raw) };
        if ok == 0 {
            return None;
        }
        return wrap(raw);
    }
    #[cfg(not(feature = "occt"))]
    {
        let _ = path;
        None
    }
}

pub fn step_write(shape: &Shape, path: impl AsRef<Path>) -> bool {
    #[cfg(feature = "occt")]
    {
        let Ok(path) = CString::new(path.as_ref().to_string_lossy().as_bytes()) else {
            return false;
        };
        return unsafe { ffi::pf_step_write(shape.raw, path.as_ptr()) != 0 };
    }
    #[cfg(not(feature = "occt"))]
    {
        let _ = (shape, path);
        false
    }
}

/// Convenience: `Shape` must not be sent across threads while OCCT is active.
pub fn is_enabled() -> bool {
    ENABLED
}
