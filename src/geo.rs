//! Caribbean geography: coastline polygons rasterized into a grid of cells,
//! land/sea queries, A* pathfinding for AI ships and map texture rendering.

use crate::ui;
use macroquad::prelude::*;
use std::collections::BinaryHeap;

pub const LON0: f32 = -98.0;
pub const LON1: f32 = -59.0;
pub const LAT0: f32 = 7.0;
pub const LAT1: f32 = 31.0;
pub const CELL: f32 = 0.1;
pub const GW: usize = 390;
pub const GH: usize = 240;
/// Pixels per cell in the map texture.
pub const TPX: usize = 4;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tile {
    Sea,
    Shallow,
    Land,
}

pub struct Geo {
    pub tiles: Vec<Tile>,
    /// Distance (in cells) from the sea for land tiles; 0 for water.
    pub inland: Vec<u8>,
    pub texture: Texture2D,
}

type P = (f32, f32);

const MAINLAND: &[P] = &[
    (-98.5, 31.5), (-81.5, 31.5), (-81.45, 30.7), (-81.3, 29.9), (-81.0, 29.2), (-80.6, 28.4), (-80.6, 28.0),
    (-80.2, 27.0), (-80.05, 26.0), (-80.2, 25.5), (-80.45, 25.2), (-80.9, 25.15), (-81.15, 25.4), (-81.4, 25.9),
    (-81.8, 26.3), (-82.1, 26.8), (-82.5, 27.4), (-82.7, 28.0), (-82.65, 28.8), (-83.0, 29.1), (-83.6, 29.9),
    (-84.2, 30.05), (-84.9, 29.7), (-85.4, 29.7), (-86.0, 30.25), (-87.0, 30.3), (-88.0, 30.35), (-88.6, 30.3),
    (-89.3, 30.25), (-89.6, 30.05), (-89.4, 29.6), (-89.1, 29.1), (-89.4, 28.95), (-90.0, 29.2), (-90.6, 29.1),
    (-91.3, 29.3), (-92.2, 29.6), (-93.3, 29.75), (-94.4, 29.5), (-94.9, 29.3), (-95.5, 28.8), (-96.4, 28.3),
    (-97.0, 27.8), (-97.35, 27.2), (-97.4, 26.4), (-97.2, 25.9), (-97.5, 25.0), (-97.75, 24.0), (-97.75, 22.9),
    (-97.8, 22.2), (-97.6, 21.6), (-97.3, 21.0), (-97.1, 20.5), (-96.5, 19.9), (-96.1, 19.2), (-95.6, 18.75),
    (-94.8, 18.5), (-94.4, 18.15), (-93.6, 18.4), (-92.7, 18.6), (-91.9, 18.65), (-91.3, 18.75), (-90.75, 19.4),
    (-90.5, 19.85), (-90.4, 20.6), (-90.3, 21.05), (-89.6, 21.3), (-88.6, 21.5), (-87.6, 21.55), (-87.05, 21.55),
    (-86.8, 21.1), (-87.0, 20.6), (-87.4, 20.2), (-87.5, 19.6), (-87.7, 18.8), (-87.85, 18.3), (-88.15, 18.0),
    (-88.2, 17.5), (-88.3, 16.8), (-88.6, 16.2), (-88.9, 15.85), (-88.2, 15.7), (-87.6, 15.85), (-86.9, 15.75),
    (-86.0, 15.9), (-85.3, 15.9), (-84.5, 15.8), (-83.75, 15.3), (-83.2, 15.0), (-83.35, 14.0), (-83.5, 12.9),
    (-83.6, 12.0), (-83.7, 11.4), (-83.65, 10.9), (-83.3, 10.4), (-82.9, 10.0), (-82.5, 9.5), (-82.1, 9.2),
    (-81.6, 8.9), (-81.0, 8.85), (-80.5, 9.1), (-79.9, 9.35), (-79.5, 9.55), (-78.9, 9.45), (-78.2, 9.25),
    (-77.7, 8.8), (-77.35, 8.6), (-77.1, 8.15), (-76.8, 8.1), (-76.75, 8.6), (-76.3, 9.2), (-75.9, 9.45),
    (-75.6, 10.1), (-75.55, 10.4), (-75.3, 10.75), (-74.85, 11.05), (-74.25, 11.2), (-73.8, 11.3), (-73.3, 11.3),
    (-72.9, 11.5), (-72.4, 11.85), (-71.95, 12.2), (-71.7, 12.4), (-71.3, 12.2), (-71.15, 11.9), (-71.4, 11.6),
    (-71.6, 10.95), (-71.2, 11.1), (-70.8, 11.3), (-70.25, 11.6), (-70.0, 12.15), (-69.85, 11.8), (-69.6, 11.42),
    (-69.0, 11.5), (-68.4, 11.0), (-68.2, 10.6), (-67.6, 10.5), (-66.9, 10.6), (-66.2, 10.6), (-65.6, 10.2),
    (-64.9, 10.05), (-64.4, 10.4), (-63.9, 10.6), (-63.0, 10.7), (-62.25, 10.7), (-61.85, 10.7), (-62.2, 10.5),
    (-62.5, 10.2), (-61.7, 9.95), (-61.0, 9.6), (-60.7, 9.2), (-60.3, 8.6), (-59.7, 8.25), (-59.2, 7.9),
    (-58.5, 7.6), (-58.5, 6.5), (-98.5, 6.5),
];

const CUBA: &[P] = &[
    (-84.95, 21.85), (-84.5, 22.2), (-84.0, 22.7), (-83.2, 22.95), (-82.36, 23.17), (-81.6, 23.15), (-81.0, 23.1),
    (-80.3, 22.95), (-79.6, 22.65), (-78.7, 22.45), (-77.8, 22.0), (-77.2, 21.65), (-76.4, 21.25), (-75.6, 21.1),
    (-74.9, 20.65), (-74.15, 20.25), (-74.5, 20.05), (-75.2, 19.95), (-75.83, 19.95), (-76.6, 19.9), (-77.7, 19.85),
    (-77.3, 20.4), (-77.8, 20.7), (-78.3, 21.3), (-78.8, 21.6), (-79.5, 21.65), (-80.0, 21.75), (-80.6, 22.05),
    (-81.3, 22.15), (-81.8, 22.25), (-82.2, 22.4), (-82.9, 22.3), (-83.4, 22.15), (-84.0, 21.95), (-84.5, 21.8),
];

const HISPANIOLA: &[P] = &[
    (-72.35, 18.55), (-73.0, 18.6), (-73.6, 18.62), (-74.45, 18.45), (-73.8, 18.1), (-73.2, 18.15), (-72.5, 18.2),
    (-71.7, 18.0), (-71.4, 17.65), (-71.0, 18.15), (-70.5, 18.2), (-69.9, 18.43), (-69.2, 18.42), (-68.8, 18.4),
    (-68.35, 18.6), (-68.9, 18.95), (-69.6, 19.15), (-69.4, 19.3), (-70.0, 19.65), (-70.7, 19.8), (-71.7, 19.88),
    (-72.2, 19.75), (-72.8, 19.93), (-73.4, 19.85), (-73.2, 19.6), (-72.8, 19.45), (-72.65, 19.1), (-72.5, 18.8),
];

const JAMAICA: &[P] = &[
    (-78.37, 18.45), (-77.9, 18.52), (-77.0, 18.47), (-76.35, 18.2), (-76.2, 17.95), (-76.8, 17.9), (-77.2, 17.72),
    (-77.8, 17.85), (-78.3, 18.2),
];

const PUERTO_RICO: &[P] = &[
    (-67.25, 18.5), (-66.0, 18.47), (-65.6, 18.38), (-65.6, 18.0), (-66.5, 17.93), (-67.2, 17.95),
];

const TRINIDAD: &[P] = &[
    (-61.92, 10.72), (-61.0, 10.85), (-60.9, 10.15), (-61.0, 10.05), (-61.9, 10.05), (-61.6, 10.3), (-61.55, 10.6),
];

/// (lon, lat, rx, ry, rotation degrees)
const ISLETS: &[(f32, f32, f32, f32, f32)] = &[
    (-78.4, 26.6, 0.6, 0.12, -10.0),
    (-77.2, 26.3, 0.15, 0.5, 20.0),
    (-78.0, 24.4, 0.3, 0.7, 10.0),
    (-77.35, 25.03, 0.17, 0.08, 0.0),
    (-76.3, 25.2, 0.1, 0.5, -30.0),
    (-75.5, 24.4, 0.08, 0.35, -20.0),
    (-76.0, 23.6, 0.08, 0.35, -40.0),
    (-75.1, 23.2, 0.07, 0.4, -30.0),
    (-74.2, 22.5, 0.2, 0.3, 0.0),
    (-73.0, 22.4, 0.2, 0.08, 0.0),
    (-73.4, 21.1, 0.35, 0.2, 0.0),
    (-71.9, 21.8, 0.35, 0.15, 0.0),
    (-81.25, 19.32, 0.2, 0.07, 0.0),
    (-86.9, 20.4, 0.08, 0.2, 30.0),
    (-86.5, 16.35, 0.3, 0.07, 10.0),
    (-82.8, 21.7, 0.3, 0.22, 0.0),
    (-72.78, 20.06, 0.22, 0.07, 0.0),
    (-73.65, 18.06, 0.12, 0.06, 0.0),
    (-73.0, 18.85, 0.3, 0.08, -15.0),
    (-64.93, 18.34, 0.11, 0.06, 0.0),
    (-64.6, 18.43, 0.1, 0.05, 0.0),
    (-64.75, 17.73, 0.2, 0.07, 0.0),
    (-63.05, 18.05, 0.09, 0.07, 0.0),
    (-62.98, 17.49, 0.06, 0.06, 0.0),
    (-62.75, 17.3, 0.12, 0.06, -40.0),
    (-62.6, 17.14, 0.06, 0.06, 0.0),
    (-61.8, 17.07, 0.12, 0.1, 0.0),
    (-61.8, 17.63, 0.08, 0.08, 0.0),
    (-62.19, 16.74, 0.06, 0.07, 0.0),
    (-61.65, 16.15, 0.13, 0.2, 0.0),
    (-61.4, 16.3, 0.15, 0.12, 0.0),
    (-61.27, 15.95, 0.07, 0.07, 0.0),
    (-61.35, 15.42, 0.1, 0.22, 0.0),
    (-61.0, 14.65, 0.13, 0.25, -30.0),
    (-60.97, 13.9, 0.08, 0.18, 0.0),
    (-61.2, 13.25, 0.07, 0.12, 0.0),
    (-59.55, 13.15, 0.1, 0.14, 0.0),
    (-61.67, 12.12, 0.08, 0.12, 0.0),
    (-60.7, 11.25, 0.2, 0.07, 30.0),
    (-64.0, 11.0, 0.28, 0.1, 0.0),
    (-68.95, 12.2, 0.25, 0.08, -35.0),
    (-68.27, 12.18, 0.12, 0.1, 0.0),
    (-70.0, 12.5, 0.13, 0.07, -40.0),
];

/// Shallow banks (lon, lat, rx, ry, rotation)
const BANKS: &[(f32, f32, f32, f32, f32)] = &[
    (-78.2, 26.6, 1.1, 0.45, -10.0),
    (-77.6, 24.2, 1.0, 1.4, 15.0),
    (-76.0, 23.8, 0.6, 1.2, -30.0),
    (-81.0, 24.8, 0.9, 0.18, 20.0),
    (-89.8, 22.0, 2.2, 0.8, 0.0),
    (-82.5, 15.0, 0.9, 1.6, 0.0),
    (-80.0, 22.9, 1.3, 0.25, -15.0),
    (-78.8, 21.2, 1.2, 0.25, -10.0),
    (-82.5, 21.8, 1.2, 0.35, 0.0),
];

fn ellipse(cx: f32, cy: f32, rx: f32, ry: f32, rot: f32) -> Vec<P> {
    let r = rot.to_radians();
    (0..16)
        .map(|i| {
            let a = i as f32 / 16.0 * std::f32::consts::TAU;
            let (x, y) = (a.cos() * rx, a.sin() * ry);
            (cx + x * r.cos() - y * r.sin(), cy + x * r.sin() + y * r.cos())
        })
        .collect()
}

fn point_in_poly(x: f32, y: f32, poly: &[P]) -> bool {
    let mut inside = false;
    let n = poly.len();
    let mut j = n - 1;
    for i in 0..n {
        let (xi, yi) = poly[i];
        let (xj, yj) = poly[j];
        if (yi > y) != (yj > y) && x < (xj - xi) * (y - yi) / (yj - yi) + xi {
            inside = !inside;
        }
        j = i;
    }
    inside
}

fn bbox(poly: &[P]) -> (f32, f32, f32, f32) {
    let mut b = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for &(x, y) in poly {
        b.0 = b.0.min(x);
        b.1 = b.1.min(y);
        b.2 = b.2.max(x);
        b.3 = b.3.max(y);
    }
    b
}

/// Convert lon/lat to fractional grid coordinates.
pub fn to_grid(lon: f32, lat: f32) -> (f32, f32) {
    ((lon - LON0) / CELL, (LAT1 - lat) / CELL)
}

pub fn from_grid(gx: f32, gy: f32) -> (f32, f32) {
    (LON0 + gx * CELL, LAT1 - gy * CELL)
}

pub fn cell_center(i: usize, j: usize) -> (f32, f32) {
    from_grid(i as f32 + 0.5, j as f32 + 0.5)
}

/// Cheap deterministic hash noise for terrain coloring.
fn hash(i: usize, j: usize, salt: u32) -> f32 {
    let mut h = (i as u32).wrapping_mul(374761393) ^ (j as u32).wrapping_mul(668265263) ^ salt.wrapping_mul(2246822519);
    h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    h ^= h >> 16;
    (h & 0xffff) as f32 / 65535.0
}

impl Geo {
    pub fn new() -> Geo {
        let mut polys: Vec<Vec<P>> = vec![
            MAINLAND.to_vec(),
            CUBA.to_vec(),
            HISPANIOLA.to_vec(),
            JAMAICA.to_vec(),
            PUERTO_RICO.to_vec(),
            TRINIDAD.to_vec(),
        ];
        for &(x, y, rx, ry, r) in ISLETS {
            polys.push(ellipse(x, y, rx, ry, r));
        }
        let banks: Vec<Vec<P>> = BANKS.iter().map(|&(x, y, rx, ry, r)| ellipse(x, y, rx, ry, r)).collect();
        let boxes: Vec<_> = polys.iter().map(|p| bbox(p)).collect();
        let bank_boxes: Vec<_> = banks.iter().map(|p| bbox(p)).collect();

        let mut tiles = vec![Tile::Sea; GW * GH];
        for j in 0..GH {
            for i in 0..GW {
                let (x, y) = cell_center(i, j);
                let mut land = false;
                for (p, b) in polys.iter().zip(&boxes) {
                    if x >= b.0 && x <= b.2 && y >= b.1 && y <= b.3 && point_in_poly(x, y, p) {
                        land = true;
                        break;
                    }
                }
                if land {
                    tiles[j * GW + i] = Tile::Land;
                    continue;
                }
                for (p, b) in banks.iter().zip(&bank_boxes) {
                    if x >= b.0 && x <= b.2 && y >= b.1 && y <= b.3 && point_in_poly(x, y, p) {
                        tiles[j * GW + i] = Tile::Shallow;
                        break;
                    }
                }
            }
        }
        // Water touching land is shallow.
        let snapshot = tiles.clone();
        for j in 0..GH {
            for i in 0..GW {
                if snapshot[j * GW + i] != Tile::Sea {
                    continue;
                }
                let mut near = false;
                for dj in -1i32..=1 {
                    for di in -1i32..=1 {
                        let (ni, nj) = (i as i32 + di, j as i32 + dj);
                        if ni >= 0 && nj >= 0 && (ni as usize) < GW && (nj as usize) < GH && snapshot[nj as usize * GW + ni as usize] == Tile::Land {
                            near = true;
                        }
                    }
                }
                if near {
                    tiles[j * GW + i] = Tile::Shallow;
                }
            }
        }
        // Inland distance via BFS from water.
        let mut inland = vec![0u8; GW * GH];
        let mut queue = std::collections::VecDeque::new();
        for j in 0..GH {
            for i in 0..GW {
                if tiles[j * GW + i] != Tile::Land {
                    queue.push_back((i, j));
                } else {
                    inland[j * GW + i] = 255;
                }
            }
        }
        while let Some((i, j)) = queue.pop_front() {
            let d = inland[j * GW + i];
            for (di, dj) in [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                let (ni, nj) = (i as i32 + di, j as i32 + dj);
                if ni < 0 || nj < 0 || ni as usize >= GW || nj as usize >= GH {
                    continue;
                }
                let k = nj as usize * GW + ni as usize;
                if inland[k] == 255 {
                    inland[k] = d.saturating_add(1).min(254);
                    queue.push_back((ni as usize, nj as usize));
                }
            }
        }
        let texture = Self::render(&tiles, &inland);
        Geo { tiles, inland, texture }
    }

    fn render(tiles: &[Tile], inland: &[u8]) -> Texture2D {
        let (w, h) = (GW * TPX, GH * TPX);
        let mut img = Image::gen_image_color(w as u16, h as u16, ui::SEA);
        for j in 0..GH {
            for i in 0..GW {
                let k = j * GW + i;
                for py in 0..TPX {
                    for px in 0..TPX {
                        let n = hash(i * TPX + px, j * TPX + py, 7);
                        let c = match tiles[k] {
                            Tile::Sea => {
                                if n > 0.97 {
                                    Color::new(0.15, 0.27, 0.68, 1.0)
                                } else {
                                    ui::SEA
                                }
                            }
                            Tile::Shallow => {
                                if n > 0.9 {
                                    Color::new(0.28, 0.5, 0.86, 1.0)
                                } else {
                                    ui::SHALLOW
                                }
                            }
                            Tile::Land => {
                                let d = inland[k];
                                let t = hash(i / 3, j / 3, 11);
                                if d <= 1 {
                                    if n > 0.5 { Color::new(0.85, 0.75, 0.45, 1.0) } else { Color::new(0.2, 0.62, 0.15, 1.0) }
                                } else if d > 14 && t > 0.35 {
                                    if n > 0.6 { Color::new(0.55, 0.45, 0.35, 1.0) } else { Color::new(0.45, 0.35, 0.25, 1.0) }
                                } else if d > 6 && t > 0.5 {
                                    if n > 0.5 { Color::new(0.35, 0.45, 0.15, 1.0) } else { Color::new(0.25, 0.4, 0.1, 1.0) }
                                } else if n > 0.75 {
                                    Color::new(0.05, 0.45, 0.08, 1.0)
                                } else {
                                    Color::new(0.1, 0.55, 0.12, 1.0)
                                }
                            }
                        };
                        img.set_pixel((i * TPX + px) as u32, (j * TPX + py) as u32, c);
                    }
                }
            }
        }
        let t = Texture2D::from_image(&img);
        t.set_filter(FilterMode::Nearest);
        t
    }

    pub fn tile_at(&self, lon: f32, lat: f32) -> Tile {
        let (gx, gy) = to_grid(lon, lat);
        if gx < 0.0 || gy < 0.0 || gx >= GW as f32 || gy >= GH as f32 {
            return Tile::Sea;
        }
        self.tiles[gy as usize * GW + gx as usize]
    }

    pub fn tile(&self, i: usize, j: usize) -> Tile {
        if i >= GW || j >= GH {
            return Tile::Sea;
        }
        self.tiles[j * GW + i]
    }

    pub fn is_water(&self, lon: f32, lat: f32) -> bool {
        self.tile_at(lon, lat) != Tile::Land
    }

    pub fn in_bounds(lon: f32, lat: f32) -> bool {
        lon > LON0 + 0.2 && lon < LON1 - 0.2 && lat > LAT0 + 0.2 && lat < LAT1 - 0.2
    }

    /// True if the straight segment between two points stays on water.
    pub fn clear_line(&self, a: Vec2, b: Vec2) -> bool {
        let d = a.distance(b);
        let steps = (d / 0.04).ceil().max(1.0) as i32;
        for s in 0..=steps {
            let p = a.lerp(b, s as f32 / steps as f32);
            if !self.is_water(p.x, p.y) {
                return false;
            }
        }
        true
    }

    /// Snap a town position onto a land cell bordering water. Returns the
    /// land position and a nearby water "anchorage" position.
    pub fn snap_town(&self, lon: f32, lat: f32) -> (Vec2, Vec2) {
        let (gx, gy) = to_grid(lon, lat);
        let (ci, cj) = (gx as i32, gy as i32);
        let mut best: Option<(f32, usize, usize)> = None;
        for r in 0..12i32 {
            for dj in -r..=r {
                for di in -r..=r {
                    let (i, j) = (ci + di, cj + dj);
                    if i < 1 || j < 1 || i as usize >= GW - 1 || j as usize >= GH - 1 {
                        continue;
                    }
                    let (i, j) = (i as usize, j as usize);
                    if self.tile(i, j) != Tile::Land {
                        continue;
                    }
                    let coastal = [(0i32, 1i32), (1, 0), (0, -1), (-1, 0)]
                        .iter()
                        .any(|(a, b)| self.tile((i as i32 + a) as usize, (j as i32 + b) as usize) != Tile::Land);
                    if !coastal {
                        continue;
                    }
                    let d = ((i as f32 + 0.5 - gx).powi(2) + (j as f32 + 0.5 - gy).powi(2)).sqrt();
                    if best.is_none_or(|b| d < b.0) {
                        best = Some((d, i, j));
                    }
                }
            }
            if best.is_some() {
                break;
            }
        }
        let (_, i, j) = best.unwrap_or((0.0, ci.max(0) as usize, cj.max(0) as usize));
        let land = cell_center(i, j);
        // Find nearest water cell for anchorage (prefer deep water nearby).
        let mut water = land;
        let mut bd = f32::MAX;
        for dj in -3i32..=3 {
            for di in -3i32..=3 {
                let (ni, nj) = (i as i32 + di, j as i32 + dj);
                if ni < 0 || nj < 0 {
                    continue;
                }
                let t = self.tile(ni as usize, nj as usize);
                if t == Tile::Land {
                    continue;
                }
                let d = (di * di + dj * dj) as f32 + if t == Tile::Shallow { 0.5 } else { 0.0 };
                if d < bd {
                    bd = d;
                    water = cell_center(ni as usize, nj as usize);
                }
            }
        }
        (vec2(land.0, land.1), vec2(water.0, water.1))
    }

    /// A* path over water between two positions. Returns simplified waypoints
    /// (excluding the start), or None if unreachable.
    pub fn find_path(&self, from: Vec2, to: Vec2) -> Option<Vec<Vec2>> {
        let (sx, sy) = to_grid(from.x, from.y);
        let (tx, ty) = to_grid(to.x, to.y);
        let clamp = |v: f32, m: usize| (v.max(0.0) as usize).min(m - 1);
        let start = (clamp(sx, GW), clamp(sy, GH));
        let goal = (clamp(tx, GW), clamp(ty, GH));
        let idx = |p: (usize, usize)| p.1 * GW + p.0;
        if self.tiles[idx(goal)] == Tile::Land {
            return None;
        }
        #[derive(PartialEq)]
        struct Node(f32, usize);
        impl Eq for Node {}
        impl PartialOrd for Node {
            fn partial_cmp(&self, o: &Self) -> Option<std::cmp::Ordering> {
                Some(self.cmp(o))
            }
        }
        impl Ord for Node {
            fn cmp(&self, o: &Self) -> std::cmp::Ordering {
                o.0.partial_cmp(&self.0).unwrap_or(std::cmp::Ordering::Equal)
            }
        }
        let mut g = vec![f32::MAX; GW * GH];
        let mut came = vec![usize::MAX; GW * GH];
        let mut heap = BinaryHeap::new();
        let h = |i: usize| {
            let (x, y) = ((i % GW) as f32, (i / GW) as f32);
            ((x - goal.0 as f32).powi(2) + (y - goal.1 as f32).powi(2)).sqrt()
        };
        let s = idx(start);
        g[s] = 0.0;
        heap.push(Node(h(s), s));
        let gi = idx(goal);
        let mut found = false;
        while let Some(Node(_, cur)) = heap.pop() {
            if cur == gi {
                found = true;
                break;
            }
            let (cx, cy) = ((cur % GW) as i32, (cur / GW) as i32);
            for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1), (-1, -1), (1, -1), (-1, 1), (1, 1)] {
                let (nx, ny) = (cx + dx, cy + dy);
                if nx < 0 || ny < 0 || nx as usize >= GW || ny as usize >= GH {
                    continue;
                }
                let n = ny as usize * GW + nx as usize;
                let t = self.tiles[n];
                if t == Tile::Land && n != gi {
                    continue;
                }
                let step = if dx != 0 && dy != 0 { 1.414 } else { 1.0 };
                let cost = step * if t == Tile::Shallow { 2.0 } else { 1.0 };
                let ng = g[cur] + cost;
                if ng < g[n] {
                    g[n] = ng;
                    came[n] = cur;
                    heap.push(Node(ng + h(n), n));
                }
            }
        }
        if !found {
            return None;
        }
        let mut cells = vec![];
        let mut c = gi;
        while c != s && c != usize::MAX {
            cells.push(c);
            c = came[c];
        }
        cells.reverse();
        let pts: Vec<Vec2> = cells
            .iter()
            .map(|&c| {
                let p = cell_center(c % GW, c / GW);
                vec2(p.0, p.1)
            })
            .collect();
        // String pulling.
        let mut out = vec![];
        let mut cur = from;
        let mut i = 0;
        while i < pts.len() {
            let mut j = pts.len() - 1;
            while j > i && !self.clear_line(cur, pts[j]) {
                j -= 1;
            }
            out.push(pts[j]);
            cur = pts[j];
            i = j + 1;
        }
        if let Some(last) = out.last_mut() {
            *last = to;
        }
        Some(out)
    }

    /// A random open-water position.
    pub fn random_sea(&self) -> Vec2 {
        loop {
            let lon = macroquad::rand::gen_range(LON0 + 1.0, LON1 - 1.0);
            let lat = macroquad::rand::gen_range(LAT0 + 1.0, LAT1 - 1.0);
            if self.tile_at(lon, lat) == Tile::Sea {
                return vec2(lon, lat);
            }
        }
    }

    /// A random coastal land cell (for buried treasure).
    pub fn random_coast_land(&self) -> Vec2 {
        loop {
            let i = macroquad::rand::gen_range(2, GW - 2);
            let j = macroquad::rand::gen_range(30, GH - 20);
            if self.tile(i, j) == Tile::Land && self.inland[j * GW + i] == 1 {
                let p = cell_center(i, j);
                return vec2(p.0, p.1);
            }
        }
    }
}
