// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! Explicit model-frame convention, separate from caller-selected camera views.
use serde::Serialize;

/// Interpretation of a native Part/Assembly's model XYZ basis.
/// This declares a default convention; it does not claim a decoded custom frame.
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
pub struct CoordinateFrame {
    /// Basis identity, without reordering or reflecting cached coordinates.
    pub basis: &'static str,
    /// Orientation of the XYZ basis.
    pub handedness: &'static str,
    /// Native Part or Assembly origin, never its bounding-box center.
    pub origin: &'static str,
    /// Default Front reference plane.
    pub front_plane: &'static str,
    /// Up direction in the standard Front view, not inferred physical gravity.
    pub front_view_up: &'static str,
    /// Direction from the model toward a standard Front-view observer.
    pub front_view_toward_observer: &'static str,
    /// Provenance of this interpretation, explicitly an assumption.
    pub provenance: &'static str,
    /// Whether named coordinate-system feature transforms have been decoded.
    pub custom_coordinate_systems_decoded: bool,
}

/// Default right-handed model XYZ frame: Front = XY, view up = +Y, outward = +Z.
/// See SOLIDWORKS Help, "Specifying Directions", Front/Right/Top plane table:
/// <https://help.solidworks.com/2021/english/SolidWorks/cworks/c_specifying_directions.htm>.
/// Native display-cache units are declared separately by `GeometryDocument::length_unit`.
pub const SOLIDWORKS_MODEL_FRAME: CoordinateFrame = CoordinateFrame {
    basis: "solidworks_model_xyz",
    handedness: "right_handed",
    origin: "document_origin",
    front_plane: "xy",
    front_view_up: "+y",
    front_view_toward_observer: "+z",
    provenance: "assumed_default_solidworks_convention",
    custom_coordinate_systems_decoded: false,
};
