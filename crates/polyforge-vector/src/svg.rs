//! Minimal SVG path import/export for VectorMode.

use bevy::prelude::*;
use crate::VectorPath;
use std::path::Path;

#[derive(Debug)]
pub enum SvgError {
    Io(std::io::Error),
    Parse(String),
}

impl std::fmt::Display for SvgError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SvgError::Io(e) => write!(f, "io: {e}"),
            SvgError::Parse(s) => write!(f, "parse: {s}"),
        }
    }
}

impl From<std::io::Error> for SvgError {
    fn from(e: std::io::Error) -> Self {
        SvgError::Io(e)
    }
}

/// Export paths as a simple SVG document (polyline / polygon).
pub fn export_svg(paths: &[VectorPath]) -> String {
    let mut min = Vec2::splat(f32::MAX);
    let mut max = Vec2::splat(f32::MIN);
    for p in paths {
        for pt in &p.points {
            min = min.min(*pt);
            max = max.max(*pt);
        }
    }
    if !min.is_finite() {
        min = Vec2::ZERO;
        max = Vec2::ONE;
    }
    let pad = 10.0;
    let w = (max.x - min.x).max(0.01) * 100.0 + pad * 2.0;
    let h = (max.y - min.y).max(0.01) * 100.0 + pad * 2.0;
    let mut s = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="{w:.1}" height="{h:.1}" viewBox="0 0 {w:.1} {h:.1}">
"#,
    );
    for path in paths {
        if path.points.is_empty() {
            continue;
        }
        let pts: String = path
            .points
            .iter()
            .map(|p| {
                let x = (p.x - min.x) * 100.0 + pad;
                // SVG Y grows down
                let y = (max.y - p.y) * 100.0 + pad;
                format!("{x:.2},{y:.2}")
            })
            .collect::<Vec<_>>()
            .join(" ");
        let stroke = "#33d9ff";
        if path.closed {
            s.push_str(&format!(
                r#"  <polygon points="{pts}" fill="none" stroke="{stroke}" stroke-width="2"/>"#
            ));
            s.push('\n');
        } else {
            s.push_str(&format!(
                r#"  <polyline points="{pts}" fill="none" stroke="{stroke}" stroke-width="2"/>"#
            ));
            s.push('\n');
        }
    }
    s.push_str("</svg>\n");
    s
}

pub fn write_svg_file(paths: &[VectorPath], path: &Path) -> Result<(), SvgError> {
    std::fs::write(path, export_svg(paths))?;
    Ok(())
}

/// Parse a very small subset: polygon/polyline points="x,y x,y ..."
pub fn import_svg(text: &str) -> Result<Vec<VectorPath>, SvgError> {
    let mut paths = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        let closed = line.contains("<polygon");
        if !line.contains("points=") {
            continue;
        }
        if let Some(start) = line.find("points=\"") {
            let rest = &line[start + 8..];
            if let Some(end) = rest.find('"') {
                let pts_str = &rest[..end];
                let mut points = Vec::new();
                for pair in pts_str.split_whitespace() {
                    let mut xy = pair.split(',');
                    if let (Some(x), Some(y)) = (xy.next(), xy.next()) {
                        if let (Ok(x), Ok(y)) = (x.parse::<f32>(), y.parse::<f32>()) {
                            // inverse of export scale (approx)
                            points.push(Vec2::new(x / 100.0, -(y / 100.0)));
                        }
                    }
                }
                if !points.is_empty() {
                    paths.push(VectorPath {
                        name: if closed { "ImportedPolygon" } else { "ImportedPolyline" }.into(),
                        points,
                        closed,
                        beziers: Vec::new(),
                        filled: false,
                        holes: Vec::new(),
                    });
                }
            }
        }
    }
    // path d attributes
    for line in text.lines() {
        if let Some(start) = line.find("d=\"") {
            let rest = &line[start + 3..];
            if let Some(end) = rest.find('"') {
                if let Ok(p) = parse_path_d(&rest[..end]) {
                    paths.push(p);
                }
            }
        }
    }
    if paths.is_empty() {
        return Err(SvgError::Parse("no polygon/polyline/path found".into()));
    }
    Ok(paths)
}

pub fn read_svg_file(path: &Path) -> Result<Vec<VectorPath>, SvgError> {
    let text = std::fs::read_to_string(path)?;
    import_svg(&text)
}


/// Parse SVG path `d` attribute supporting M/L/H/V/C/Z (absolute + relative).
pub fn parse_path_d(d: &str) -> Result<crate::VectorPath, SvgError> {
    use crate::{BezierSegment, VectorPath};
    let mut points = Vec::new();
    let mut beziers = Vec::new();
    let mut closed = false;
    let mut cur = Vec2::ZERO;
    let mut start = Vec2::ZERO;
    let tokens = tokenize_path(d);
    let mut i = 0;
    let mut cmd = 'M';
    while i < tokens.len() {
        if tokens[i].len() == 1 && tokens[i].chars().next().unwrap().is_ascii_alphabetic() {
            cmd = tokens[i].chars().next().unwrap();
            i += 1;
            continue;
        }
        match cmd {
            'M' | 'm' => {
                let rel = cmd == 'm';
                let x = tokens[i].parse::<f32>().map_err(|_| SvgError::Parse("x".into()))?;
                let y = tokens[i + 1].parse::<f32>().map_err(|_| SvgError::Parse("y".into()))?;
                cur = if rel { cur + Vec2::new(x, y) } else { Vec2::new(x, y) };
                start = cur;
                points.push(cur);
                i += 2;
                cmd = if rel { 'l' } else { 'L' };
            }
            'L' | 'l' => {
                let rel = cmd == 'l';
                let x = tokens[i].parse::<f32>().map_err(|_| SvgError::Parse("x".into()))?;
                let y = tokens[i + 1].parse::<f32>().map_err(|_| SvgError::Parse("y".into()))?;
                let n = if rel { cur + Vec2::new(x, y) } else { Vec2::new(x, y) };
                points.push(n);
                cur = n;
                i += 2;
            }
            'H' | 'h' => {
                let rel = cmd == 'h';
                let x = tokens[i].parse::<f32>().map_err(|_| SvgError::Parse("x".into()))?;
                let n = Vec2::new(if rel { cur.x + x } else { x }, cur.y);
                points.push(n);
                cur = n;
                i += 1;
            }
            'V' | 'v' => {
                let rel = cmd == 'v';
                let y = tokens[i].parse::<f32>().map_err(|_| SvgError::Parse("y".into()))?;
                let n = Vec2::new(cur.x, if rel { cur.y + y } else { y });
                points.push(n);
                cur = n;
                i += 1;
            }
            'C' | 'c' => {
                let rel = cmd == 'c';
                if i + 5 >= tokens.len() {
                    return Err(SvgError::Parse("cubic needs 6 numbers".into()));
                }
                let nums: Vec<f32> = tokens[i..i + 6]
                    .iter()
                    .map(|s| s.parse::<f32>())
                    .collect::<Result<_, _>>()
                    .map_err(|_| SvgError::Parse("cubic num".into()))?;
                let (c1, c2, p3) = if rel {
                    (
                        cur + Vec2::new(nums[0], nums[1]),
                        cur + Vec2::new(nums[2], nums[3]),
                        cur + Vec2::new(nums[4], nums[5]),
                    )
                } else {
                    (
                        Vec2::new(nums[0], nums[1]),
                        Vec2::new(nums[2], nums[3]),
                        Vec2::new(nums[4], nums[5]),
                    )
                };
                beziers.push(BezierSegment {
                    p0: cur,
                    p1: c1,
                    p2: c2,
                    p3,
                });
                cur = p3;
                points.push(p3);
                i += 6;
            }
            'Z' | 'z' => {
                closed = true;
                cur = start;
            }
            _ => {
                i += 1;
            }
        }
    }
    Ok(VectorPath {
        name: "PathD".into(),
        points,
        closed,
        beziers,
        filled: false,
        holes: Vec::new(),
    })
}

fn tokenize_path(d: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for ch in d.chars() {
        if ch.is_ascii_alphabetic() {
            if !cur.is_empty() {
                out.push(cur.clone());
                cur.clear();
            }
            out.push(ch.to_string());
        } else if ch == ',' || ch.is_whitespace() {
            if !cur.is_empty() {
                out.push(cur.clone());
                cur.clear();
            }
        } else {
            cur.push(ch);
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}
