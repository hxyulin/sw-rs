// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! Single viewer-defined joint relative to a stationary assembly frame.

use std::{error::Error, fmt};

/// One allowed motion coordinate, independent of native mate evidence.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JointKind {
    /// Rotation about an assembly-frame axis; coordinates are radians.
    Hinge,
    /// Translation along an assembly-frame axis; coordinates are metres.
    Slider,
}

/// User-defined axis, pivot and travel limits in the saved assembly frame.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct Joint {
    /// Allowed motion type.
    pub kind: JointKind,
    /// Nonzero assembly-frame direction, normalized when evaluated.
    pub axis: [f64; 3],
    /// Assembly-frame pivot in metres; ignored for slider translation.
    pub pivot: [f64; 3],
    /// Inclusive lower coordinate limit, in radians or metres.
    pub lower: f64,
    /// Inclusive upper coordinate limit, in radians or metres.
    pub upper: f64,
}

/// Invalid joint definition or requested coordinate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MotionError {
    /// A coordinate, bound, axis or pivot is non-finite.
    NonFinite,
    /// The direction has zero length or cannot be normalized.
    InvalidAxis,
    /// Lower travel bound exceeds the upper bound.
    InvalidLimits,
    /// Requested coordinate exceeds the inclusive travel bounds.
    OutsideLimits,
}
impl fmt::Display for MotionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::NonFinite => "joint values must be finite",
            Self::InvalidAxis => "joint axis must have a finite nonzero length",
            Self::InvalidLimits => "lower travel limit exceeds upper limit",
            Self::OutsideLimits => "motion coordinate is outside travel limits",
        })
    }
}
impl Error for MotionError {}

impl Joint {
    /// Return a column-major rigid delta from the saved pose to the requested pose.
    ///
    /// Apply the rotation to normals and the full transform to assembly-frame points.
    /// This evaluates one joint; it does not solve or validate other assembly mates.
    ///
    /// # Errors
    /// Returns an error for invalid definitions, out-of-range coordinates or overflow.
    pub fn transform(&self, coordinate: f64) -> Result<[f64; 16], MotionError> {
        if self
            .axis
            .iter()
            .chain(&self.pivot)
            .chain([&self.lower, &self.upper, &coordinate])
            .any(|v| !v.is_finite())
        {
            return Err(MotionError::NonFinite);
        }
        if self.lower > self.upper {
            return Err(MotionError::InvalidLimits);
        }
        if coordinate < self.lower || coordinate > self.upper {
            return Err(MotionError::OutsideLimits);
        }
        let length = self.axis[0].hypot(self.axis[1]).hypot(self.axis[2]);
        if length == 0. || !length.is_finite() {
            return Err(MotionError::InvalidAxis);
        }
        let axis = self.axis.map(|v| v / length);
        let mut matrix = [
            1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.,
        ];
        match self.kind {
            JointKind::Slider => {
                for i in 0..3 {
                    matrix[12 + i] = axis[i] * coordinate;
                }
            }
            JointKind::Hinge => {
                let (sin, cos) = coordinate.sin_cos();
                let [x, y, z] = axis;
                // Rodrigues rotation, stored by columns.
                matrix[..12].copy_from_slice(&[
                    cos + x * x * (1. - cos),
                    y * x * (1. - cos) + z * sin,
                    z * x * (1. - cos) - y * sin,
                    0.,
                    x * y * (1. - cos) - z * sin,
                    cos + y * y * (1. - cos),
                    z * y * (1. - cos) + x * sin,
                    0.,
                    x * z * (1. - cos) + y * sin,
                    y * z * (1. - cos) - x * sin,
                    cos + z * z * (1. - cos),
                    0.,
                ]);
                for i in 0..3 {
                    matrix[12 + i] = self.pivot[i]
                        - (0..3)
                            .map(|j| matrix[j * 4 + i] * self.pivot[j])
                            .sum::<f64>();
                }
            }
        }
        if matrix.iter().any(|v| !v.is_finite()) {
            return Err(MotionError::NonFinite);
        }
        Ok(matrix)
    }
}

/// Pitch crank and translating coupler of an authored parallelogram linkage.
///
/// The head rotates by the same coordinate about its separate, stationary pivot.
/// This holds the folding arm in its saved pose; it does not solve native mates.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct Parallelogram {
    /// Assembly-frame direction shared by the head and crank hinge axes.
    pub axis: [f64; 3],
    /// Stationary crank pivot in metres.
    pub crank_pivot: [f64; 3],
    /// Saved crank-to-coupler connection point in metres.
    pub crank_point: [f64; 3],
    /// Inclusive lower coordinate limit in radians.
    pub lower: f64,
    /// Inclusive upper coordinate limit in radians.
    pub upper: f64,
}

/// Assembly-frame deltas for a parallelogram's crank and coupler.
#[derive(Clone, Copy, Debug, serde::Serialize)]
pub struct ParallelogramPose {
    /// Crank rotation, including translation about its stationary pivot.
    pub crank: [f64; 16],
    /// Coupler translation, preserving its saved orientation.
    pub coupler: [f64; 16],
}

impl Parallelogram {
    /// Evaluate a linkage pose while keeping both crank lengths and the coupler length constant.
    ///
    /// # Errors
    /// Returns an error for invalid definitions, coordinates or arithmetic overflow.
    pub fn transforms(&self, coordinate: f64) -> Result<ParallelogramPose, MotionError> {
        if self.crank_point.iter().any(|v| !v.is_finite()) {
            return Err(MotionError::NonFinite);
        }
        let crank = Joint {
            kind: JointKind::Hinge,
            axis: self.axis,
            pivot: self.crank_pivot,
            lower: self.lower,
            upper: self.upper,
        }
        .transform(coordinate)?;
        let mut coupler = [
            1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.,
        ];
        for i in 0..3 {
            coupler[12 + i] = crank[12 + i] - self.crank_point[i]
                + (0..3)
                    .map(|j| crank[4 * j + i] * self.crank_point[j])
                    .sum::<f64>();
        }
        if coupler.iter().any(|v| !v.is_finite()) {
            return Err(MotionError::NonFinite);
        }
        Ok(ParallelogramPose { crank, coupler })
    }
}

/// Evaluate an authored parallelogram request, returning crank then coupler matrices.
///
/// # Errors
/// Returns an error for malformed JSON, invalid definitions or coordinates.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn linkage_transform(request: &str) -> Result<Vec<f64>, wasm_bindgen::JsError> {
    #[derive(serde::Deserialize)]
    struct Request {
        linkage: Parallelogram,
        coordinate: f64,
    }
    let request: Request = serde_json::from_str(request)?;
    let pose = request.linkage.transforms(request.coordinate)?;
    Ok(pose.crank.into_iter().chain(pose.coupler).collect())
}

/// Evaluate a JSON joint request for browser preview controls.
///
/// # Errors
/// Returns an error for malformed JSON, invalid joint definitions or coordinates.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn joint_transform(request: &str) -> Result<Vec<f64>, wasm_bindgen::JsError> {
    #[derive(serde::Deserialize)]
    struct Request {
        joint: Joint,
        coordinate: f64,
    }
    let request: Request = serde_json::from_str(request)?;
    Ok(request.joint.transform(request.coordinate)?.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn joint(kind: JointKind) -> Joint {
        Joint {
            kind,
            axis: [0., 0., 2.],
            pivot: [1., 2., 3.],
            lower: -10.,
            upper: 10.,
        }
    }
    fn point(m: [f64; 16], p: [f64; 3]) -> [f64; 3] {
        std::array::from_fn(|i| m[i] * p[0] + m[4 + i] * p[1] + m[8 + i] * p[2] + m[12 + i])
    }
    fn close(actual: [f64; 3], expected: [f64; 3]) {
        for i in 0..3 {
            assert!(
                (actual[i] - expected[i]).abs() < 1e-12,
                "actual {actual:?}, expected {expected:?}"
            );
        }
    }
    #[test]
    fn hinge_preserves_axis_and_rotates_about_offset_pivot() {
        let j = joint(JointKind::Hinge);
        let m = j.transform(std::f64::consts::FRAC_PI_2).unwrap();
        close(point(m, j.pivot), j.pivot);
        close(point(m, [1., 2., 8.]), [1., 2., 8.]);
        close(point(m, [2., 2., 3.]), [1., 3., 3.]);
        close(
            point(
                j.transform(-std::f64::consts::FRAC_PI_2).unwrap(),
                point(m, [2., 2., 3.]),
            ),
            [2., 2., 3.],
        );
    }
    #[test]
    fn slider_normalizes_axis_and_preserves_orientation() {
        let j = joint(JointKind::Slider);
        close(
            point(j.transform(0.025).unwrap(), [1., 2., 3.]),
            [1., 2., 3.025],
        );
        let m = j.transform(-0.025).unwrap();
        close(point(m, [1., 2., 3.]), [1., 2., 2.975]);
        assert_eq!(&m[..12], &[1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0.]);
    }
    #[test]
    fn limits_and_invalid_inputs_are_rejected() {
        let mut j = joint(JointKind::Hinge);
        assert!(j.transform(j.lower).is_ok());
        assert!(j.transform(j.upper).is_ok());
        assert_eq!(j.transform(11.), Err(MotionError::OutsideLimits));
        assert_eq!(j.transform(f64::NAN), Err(MotionError::NonFinite));
        j.axis = [0.; 3];
        assert_eq!(j.transform(0.), Err(MotionError::InvalidAxis));
        j.axis = [f64::INFINITY, 0., 1.];
        assert_eq!(j.transform(0.), Err(MotionError::NonFinite));
        j = joint(JointKind::Slider);
        j.lower = 20.;
        assert_eq!(j.transform(0.), Err(MotionError::InvalidLimits));
    }
    #[test]
    fn zero_coordinate_keeps_saved_pose() {
        for kind in [JointKind::Hinge, JointKind::Slider] {
            close(
                point(joint(kind).transform(0.).unwrap(), [4., 5., 6.]),
                [4., 5., 6.],
            );
        }
    }
    #[test]
    fn parallelogram_keeps_both_connections_closed_and_coupler_orientation() {
        let a = [-0.033, 0., 0.];
        let b = [-0.158, 0., 0.];
        let c = [-0.012, 0.028, 0.];
        let d = [c[0] + b[0] - a[0], c[1], 0.];
        let linkage = Parallelogram {
            axis: [0., 0., 1.],
            crank_pivot: a,
            crank_point: c,
            lower: -1.,
            upper: 1.,
        };
        for angle in [-0.5, 0., 0.5] {
            let pose = linkage.transforms(angle).unwrap();
            let head = Joint {
                kind: JointKind::Hinge,
                axis: linkage.axis,
                pivot: b,
                lower: -1.,
                upper: 1.,
            }
            .transform(angle)
            .unwrap();
            close(point(pose.crank, a), a);
            close(point(head, b), b);
            close(point(pose.crank, c), point(pose.coupler, c));
            close(point(head, d), point(pose.coupler, d));
            assert_eq!(
                &pose.coupler[..12],
                &[1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0.]
            );
        }
        assert!(matches!(
            linkage.transforms(2.),
            Err(MotionError::OutsideLimits)
        ));
        assert!(matches!(
            Parallelogram {
                crank_point: [f64::NAN, 0., 0.],
                ..linkage
            }
            .transforms(0.),
            Err(MotionError::NonFinite)
        ));
    }
}
