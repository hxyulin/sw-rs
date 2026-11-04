// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! Approximate arc-length coordinates for checked axis-aligned cylindrical cache faces.
use sw_rs::Tessellation;

pub(crate) struct Cylinder {
    axis: usize,
    radial: [usize; 2],
    center: [f64; 2],
    radius: f64,
    cut: f64,
}
impl Cylinder {
    pub(super) fn fit(mesh: &Tessellation) -> Option<Self> {
        // Cache coordinates use millimetres; texture coordinates use metres.
        let points: Vec<_> = mesh.vertices.iter().map(|p| p.map(|v| v / 1000.)).collect();
        if points.len() < 6 || points.iter().flatten().any(|v| !v.is_finite()) {
            return None;
        }
        for axis in 0..3 {
            let radial: Vec<_> = (0..3).filter(|i| *i != axis).collect();
            let radial = [radial[0], radial[1]];
            let low = points.iter().map(|p| p[axis]).fold(f64::INFINITY, f64::min);
            let high = points
                .iter()
                .map(|p| p[axis])
                .fold(f64::NEG_INFINITY, f64::max);
            if high - low < 1e-9 {
                continue;
            }
            let a = [points[0][radial[0]], points[0][radial[1]]];
            let distance =
                |p: &[f64; 3]| (p[radial[0]] - a[0]).powi(2) + (p[radial[1]] - a[1]).powi(2);
            let b = points
                .iter()
                .max_by(|p, q| distance(p).total_cmp(&distance(q)))?;
            let b = [b[radial[0]] - a[0], b[radial[1]] - a[1]];
            let area = |p: &[f64; 3]| b[0] * (p[radial[1]] - a[1]) - b[1] * (p[radial[0]] - a[0]);
            let c = points
                .iter()
                .max_by(|p, q| area(p).abs().total_cmp(&area(q).abs()))?;
            let determinant = 2. * area(c);
            let b2 = b[0] * b[0] + b[1] * b[1];
            if determinant.abs() < b2 * 1e-5 {
                continue;
            }
            let c = [c[radial[0]] - a[0], c[radial[1]] - a[1]];
            let c2 = c[0] * c[0] + c[1] * c[1];
            let offset = [
                (b2 * c[1] - c2 * b[1]) / determinant,
                (b[0] * c2 - c[0] * b2) / determinant,
            ];
            let center = [a[0] + offset[0], a[1] + offset[1]];
            let radius = offset[0].hypot(offset[1]);
            if !radius.is_finite() || radius < 1e-8 {
                continue;
            }
            let tolerance = (radius * 1e-4).max(1e-9);
            if points.iter().any(|p| {
                ((p[radial[0]] - center[0]).hypot(p[radial[1]] - center[1]) - radius).abs()
                    > tolerance
            }) {
                continue;
            }
            let axial_normals = mesh.triangles.iter().all(|t| {
                let p = t.map(|i| points[i as usize]);
                let a = std::array::from_fn::<_, 3, _>(|i| p[1][i] - p[0][i]);
                let b = std::array::from_fn::<_, 3, _>(|i| p[2][i] - p[0][i]);
                let n = [
                    a[1] * b[2] - a[2] * b[1],
                    a[2] * b[0] - a[0] * b[2],
                    a[0] * b[1] - a[1] * b[0],
                ];
                n[axis].abs() <= n.iter().map(|x| x * x).sum::<f64>().sqrt() * 1e-5
            });
            if !axial_normals {
                continue;
            }
            let mut angles: Vec<_> = points
                .iter()
                .map(|p| {
                    (p[radial[1]] - center[1])
                        .atan2(p[radial[0]] - center[0])
                        .rem_euclid(std::f64::consts::TAU)
                })
                .collect();
            angles.sort_by(f64::total_cmp);
            angles.dedup_by(|a, b| (*a - *b).abs() < 1e-8);
            if angles.len() < 3 {
                continue;
            }
            // Put the unwrap cut in the largest empty arc. A closed cylinder
            // necessarily retains one seam; an open fillet does not acquire one.
            let (gap, start) = (0..angles.len())
                .map(|i| {
                    let next = angles[(i + 1) % angles.len()]
                        + if i + 1 == angles.len() {
                            std::f64::consts::TAU
                        } else {
                            0.
                        };
                    (next - angles[i], angles[i])
                })
                .max_by(|a, b| a.0.total_cmp(&b.0))?;
            return Some(Self {
                axis,
                radial,
                center,
                radius,
                cut: (start + gap / 2.).rem_euclid(std::f64::consts::TAU),
            });
        }
        None
    }
    pub(super) fn coordinates(&self, points: [[f64; 3]; 3], center: [f32; 3]) -> [[f64; 2]; 3] {
        let mut angles = points.map(|p| {
            let angle = (p[self.radial[1]] - self.center[1])
                .atan2(p[self.radial[0]] - self.center[0])
                .rem_euclid(std::f64::consts::TAU);
            if angle < self.cut {
                angle + std::f64::consts::TAU
            } else {
                angle
            }
        });
        // Do not interpolate across an entire circumference at the single seam.
        let low = angles.into_iter().fold(f64::INFINITY, f64::min);
        if angles.into_iter().fold(f64::NEG_INFINITY, f64::max) - low > std::f64::consts::PI {
            for angle in &mut angles {
                if *angle - low > std::f64::consts::PI {
                    *angle -= std::f64::consts::TAU;
                }
            }
        }
        std::array::from_fn(|i| {
            [
                self.radius * angles[i],
                points[i][self.axis] - f64::from(center[self.axis]),
            ]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn arc() -> Tessellation {
        let mut vertices = Vec::new();
        for degrees in [20_f64, 40., 60., 80., 100.] {
            let a = degrees.to_radians();
            for z in [0., 2.] {
                vertices.push([10. + 3. * a.cos(), -5. + 3. * a.sin(), z]);
            }
        }
        let triangles = (0..4)
            .flat_map(|i| {
                let i = i * 2;
                [[i, i + 2, i + 1], [i + 2, i + 3, i + 1]]
            })
            .collect();
        Tessellation {
            source_stream: String::new(),
            source_offset: 0,
            metadata_span: sw_rs::ByteSpan {
                offset: 0,
                length: 0,
            },
            vertices,
            triangles,
            strip_lengths: Vec::new(),
            normals: Vec::new(),
        }
    }
    #[test]
    fn arc_length_mapping_keeps_shared_vertices_continuous_across_projection_axes() {
        let mesh = arc();
        let cylinder = Cylinder::fit(&mesh).expect("axis-aligned cylindrical fillet");
        let mut mapped = std::collections::BTreeMap::new();
        for t in &mesh.triangles {
            let points = t.map(|i| mesh.vertices[i as usize].map(|v| v / 1000.));
            let uv = cylinder.coordinates(points, [0.; 3]);
            for (i, uv) in t.iter().zip(uv) {
                if let Some(previous) = mapped.insert(*i, uv) {
                    assert_eq!(
                        uv, previous,
                        "shared vertex {i} must not jump when its neighbors cross 45 degrees"
                    );
                }
            }
        }
        let a = mapped[&0];
        let b = mapped[&8];
        assert!(
            (b[0] - a[0] - 0.003 * 80_f64.to_radians()).abs() < 1e-12,
            "distance follows arc length in metres"
        );
        assert!(
            (mapped[&1][1] - a[1] - 0.002).abs() < 1e-12,
            "axial distance stays in metres"
        );
    }
    #[test]
    fn planes_and_non_circular_curves_do_not_gain_cylinder_coordinates() {
        let mut mesh = arc();
        for p in &mut mesh.vertices {
            p[0] *= 1.5;
        }
        assert!(
            Cylinder::fit(&mesh).is_none(),
            "ellipse is not a circular cylinder"
        );
        for (i, p) in mesh.vertices.iter_mut().enumerate() {
            p[0] = 0.;
            p[1] = (i / 2) as f64;
        }
        assert!(
            Cylinder::fit(&mesh).is_none(),
            "planar face must retain its planar mapping"
        );
    }
}
