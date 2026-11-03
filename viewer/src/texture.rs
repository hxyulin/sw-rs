// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! Supplied bitmap selection and approximate Part-local dominant-plane projection.
use sw_rs::{
    PresentationData,
    archive::{FileSet, Resolution},
};

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
}
impl Projection {
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
        let (sin, cos) = self.rotation.sin_cos();
        [
            ((u * cos - v * sin) * self.scale[0] + 0.5) as f32,
            ((u * sin + v * cos) * self.scale[1] + 0.5) as f32,
        ]
    }
}

pub(crate) fn mesh_projection(
    presentation: &PresentationData,
    mesh: &sw_rs::Tessellation,
    configuration: &str,
    owner: &str,
    files: &FileSet,
) -> Result<Option<Projection>, String> {
    let Some(native) = crate::saved_appearance::mesh_native(presentation, mesh, configuration)
    else {
        return Ok(None);
    };
    native_projection(native, owner, files)
}

fn native_projection(
    native: &sw_rs::NativeAppearance,
    owner: &str,
    files: &FileSet,
) -> Result<Option<Projection>, String> {
    let Some(extension) = native.extension.as_ref() else {
        return Ok(None);
    };
    let Some(parameters) = extension.bitmap_texture_parameters() else {
        return Ok(None);
    };
    // The projection-coupled discriminator is not a planar repeat-size profile.
    if extension.shader_parameters.discriminator != 3
        || parameters
            .reciprocal_scale_per_metre
            .iter()
            .any(|s| *s <= 0.)
    {
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
                .color_rgb
                .unwrap_or([255; 3])
                .map(|v| f32::from(v) / 255.),
        },
        center: parameters.center_metres,
        scale: parameters.reciprocal_scale_per_metre,
        rotation: parameters.stored_rotation_degrees.to_radians(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
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
