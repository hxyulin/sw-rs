// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! Supplied bitmap selection and approximate Part-local dominant-plane projection.
mod cylinder;

use sw_rs::archive::{FileSet, Resolution};

/// Supplied image selected by a supported cache or document appearance binding.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct BitmapTexture {
    /// Normalized virtual path, never a host file path or URL.
    pub path: String,
    /// Saved swatch, using white when no RGB swatch was decoded.
    pub base_color: [f32; 3],
}

pub(crate) struct Projection {
    pub image: BitmapTexture,
    center: [f32; 3],
    scale: [f64; 2],
    rotation: f64,
    cylindrical_faces: bool,
}
impl Projection {
    pub(crate) fn cylinder(&self, mesh: &sw_rs::Tessellation) -> Option<cylinder::Cylinder> {
        self.cylindrical_faces
            .then(|| cylinder::Cylinder::fit(mesh))
            .flatten()
    }
    pub(crate) fn triangle_uv(
        &self,
        points: [[f64; 3]; 3],
        normal: [f64; 3],
        cylinder: Option<&cylinder::Cylinder>,
    ) -> [[f32; 2]; 3] {
        if let Some(cylinder) = cylinder {
            return cylinder
                .coordinates(points, self.center)
                .map(|uv| self.transform(uv));
        }
        points.map(|p| self.uv(p, normal))
    }
    fn transform(&self, [u, v]: [f64; 2]) -> [f32; 2] {
        let (sin, cos) = self.rotation.sin_cos();
        [
            ((u * cos - v * sin) * self.scale[0] + 0.5) as f32,
            ((u * sin + v * cos) * self.scale[1] + 0.5) as f32,
        ]
    }
    pub(crate) fn uv(&self, position: [f64; 3], normal: [f64; 3]) -> [f32; 2] {
        let p = std::array::from_fn::<_, 3, _>(|i| position[i] - f64::from(self.center[i]));
        let a = normal.map(f64::abs);
        let [u, v] = if a[2] >= a[0] && a[2] >= a[1] {
            [p[0], p[1]]
        } else if a[1] >= a[0] {
            [p[0], p[2]]
        } else {
            [p[1], p[2]]
        };
        self.transform([u, v])
    }
}

// Carbon uses the same saved scalar locations as the controlled bitmap profile.
// Reusing them is a viewer approximation, not a native mapping claim.
fn projection_parameters(
    extension: &sw_rs::NativeAppearanceExtension,
) -> Option<([f64; 2], f64, [f32; 3])> {
    if let Some(p) = extension.bitmap_texture_parameters() {
        return Some((
            p.reciprocal_scale_per_metre,
            p.stored_rotation_degrees,
            p.center_metres,
        ));
    }
    let block = extension.legacy_texture_block.as_ref()?;
    let shader = &extension.shader_parameters;
    if extension.shader_identifier.value != "carbonfiberepoxy"
        || !matches!(
            (
                shader.selectors,
                block.selectors.as_slice(),
                shader.discriminator
            ),
            ([1, 1, 3, 0], [0, 1, 0, 1, 0, 1] | [1, 0, 1, 0, 1], 3)
                | ([1, 1, 27, 0], [0, 1, 0, 1, 1, 1] | [1, 0, 1, 1, 1], 2)
                | ([1, 1, 4, 0], [0, 1, 1, 0, 1, 1] | [1, 1, 0, 1, 1], 2)
        )
        || shader.mode != 4
        || block.reference_flags != 1
    {
        return None;
    }
    let scale = [block.values[0], block.values[1]];
    let rotation = block.values[12];
    let center = [shader.values[6], shader.values[7], shader.values[8]];
    if !scale.iter().all(|v| v.is_finite() && *v > 0.)
        || !rotation.is_finite()
        || !center.iter().all(|v| v.is_finite())
    {
        return None;
    }
    Some((scale, rotation, center))
}

pub(crate) fn native_projection(
    native: &sw_rs::NativeAppearance,
    owner: &str,
    files: &FileSet,
) -> Result<Option<Projection>, String> {
    let Some(extension) = native.extension.as_ref() else {
        return Ok(None);
    };
    let Some((scale, rotation, center)) = projection_parameters(extension) else {
        return Ok(None);
    };
    // The projection-coupled discriminator is not a planar repeat-size profile.
    if extension.shader_parameters.discriminator == 6 || scale.iter().any(|s| *s <= 0.) {
        return Ok(None);
    }
    let Some(block) = extension.legacy_texture_block.as_ref() else {
        return Ok(None);
    };
    let path = match files.resolve_texture(owner, &block.reference.value) {
        Resolution::Found(path) => path,
        Resolution::Missing => {
            return Err(format!(
                "{owner}: referenced bitmap is not supplied in the archive"
            ));
        }
        Resolution::Ambiguous(_) => {
            return Err(format!(
                "{owner}: referenced bitmap matches multiple archive images"
            ));
        }
    };
    Ok(Some(Projection {
        image: BitmapTexture {
            path,
            base_color: native
                .render_color_rgb()
                .unwrap_or([255; 3])
                .map(|v| f32::from(v) / 255.),
        },
        center,
        scale,
        rotation: rotation.to_radians(),
        cylindrical_faces: extension.shader_identifier.value == "carbonfiberepoxy",
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn carbon_profile_is_opt_in_and_requires_valid_mapping_values() {
        let text = |value: &str| sw_rs::NativeString {
            offset: 0,
            value: value.into(),
        };
        let mut extension = sw_rs::NativeAppearanceExtension {
            legacy_texture_block: Some(sw_rs::NativeTextureBlock {
                values: [25., 50., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 90.],
                reference: text("weave.jpg"),
                reference_flags: 1,
                additional_values: [0.; 2],
                selectors: vec![0, 1, 0, 1, 0, 1],
            }),
            shader_parameters: sw_rs::NativeShaderParameters {
                label: text(""),
                selectors: [1, 1, 3, 0],
                offset: 0,
                discriminator: 3,
                packed_word: 0,
                values: [0.; 10],
                mode: 4,
                parameters: [0.; 5],
                flags: [0; 3],
                appearance_word: 0,
                finish_values: [0.; 2],
            },
            shader_identifier: text("carbonfiberepoxy"),
            appearance_reference: text("weave.p2m"),
            surface_finish_reference: text(""),
            mapping_offset: 0,
            mapping_scalars: [0.; 2],
            mapping_flags: [0; 3],
            mapping_values: [0.; 14],
            prefix_end: 0,
        };
        assert!(
            extension.bitmap_texture_parameters().is_none(),
            "core mapping remains unverified"
        );
        assert_eq!(
            projection_parameters(&extension),
            Some(([25., 50.], 90., [0.; 3]))
        );
        extension.shader_parameters.mode = 1;
        assert!(projection_parameters(&extension).is_none());
        extension.shader_parameters.mode = 4;
        extension.legacy_texture_block.as_mut().unwrap().values[0] = f64::NAN;
        assert!(projection_parameters(&extension).is_none());
    }
    #[test]
    fn projection_uses_metres_and_centered_repeat_scale_on_each_plane() {
        let mut projection = Projection {
            image: BitmapTexture {
                path: "grid.png".into(),
                base_color: [1.; 3],
            },
            center: [0.; 3],
            scale: [25., 50.],
            rotation: 0.,
            cylindrical_faces: false,
        };
        assert_eq!(projection.uv([0., 0., 0.], [0., 0., 1.]), [0.5, 0.5]);
        assert_eq!(projection.uv([0.04, 0.02, 0.], [0., 0., 1.]), [1.5, 1.5]);
        assert_eq!(projection.uv([0.04, 0., 0.02], [0., 1., 0.]), [1.5, 1.5]);
        assert_eq!(projection.uv([0., 0.04, 0.02], [1., 0., 0.]), [1.5, 1.5]);
        projection.rotation = std::f64::consts::FRAC_PI_2;
        let uv = projection.uv([0.04, 0., 0.], [0., 0., 1.]);
        assert!((uv[0] - 0.5).abs() < 1e-6 && (uv[1] - 2.5).abs() < 1e-6);
    }
}
