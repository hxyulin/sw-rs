// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
// Adapted from cadmpeg-codec-sldprt 0.5.3+sldkit.4, modified by sldkit.
// See NOTICE-cadmpeg.md and LICENSE-Apache-2.0 for provenance and terms.
//! Portable cached descriptor tables; no B-Rep/ownership inference.
use serde::Serialize;
use std::collections::{HashMap, HashSet};
const FACE_TESSELLATION_CLASS: &[u8] = b"uoTempFaceTessData_c";
#[derive(Debug, Clone, Default, Serialize)]
pub(crate) struct Mesh {
    pub vertices: Vec<[f64; 3]>,
    pub triangles: Vec<[u32; 3]>,
    pub strip_lengths: Vec<u32>,
    pub normals: Vec<[f64; 3]>,
}
#[derive(Debug, Clone)]
struct TessellationChannel {
    item_size: u32,
    kind: u32,
    flags: u32,
    count: u32,
    data: Vec<u8>,
}
#[derive(Debug, Clone, Copy)]
struct ByteRange {
    start: usize,
    end: usize,
}
struct DisplayFace {
    mesh: Mesh,
    table_index: usize,
    table: ByteRange,
    metadata: ByteRange,
}
struct ClassInterval {
    name: String,
    class_offset: usize,
    content: ByteRange,
}
struct View;
impl View {
    fn u16_le_at(data: &[u8], at: usize) -> Option<u16> {
        Some(u16::from_le_bytes(
            data.get(at..at.checked_add(2)?)?.try_into().ok()?,
        ))
    }
    fn u32_le_at(data: &[u8], at: usize) -> Option<u32> {
        Some(u32::from_le_bytes(
            data.get(at..at.checked_add(4)?)?.try_into().ok()?,
        ))
    }
    fn f32_le_at(data: &[u8], at: usize) -> Option<f32> {
        Some(f32::from_bits(Self::u32_le_at(data, at)?))
    }
}
fn point3(x: f64, y: f64, z: f64) -> [f64; 3] {
    [x, y, z]
}
mod compact_face {
    pub const LEN: usize = 8;
    pub const TRIANGLE_COUNT: usize = 0;
    pub const STRIP_COUNT: usize = 4;
}
mod extended_face {
    pub const LEN: usize = 40;
    pub const FORM: usize = 8;
    pub const ZERO_AT_12: usize = 12;
    pub const ZERO_AT_16: usize = 16;
    pub const FORM_TOKEN: usize = 20;
    pub const ZERO_TAIL: usize = 24;
}
fn class_intervals(payload: &[u8]) -> Vec<ClassInterval> {
    let declarations: Vec<_> = payload
        .windows(4)
        .enumerate()
        .filter(|(_, w)| *w == [255, 255, 1, 0])
        .filter_map(|(at, _)| {
            let len = View::u16_le_at(payload, at + 4)? as usize;
            if len == 0 {
                return None;
            }
            let bytes = payload.get(at + 6..at + 6 + len)?;
            if !bytes
                .iter()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
            {
                return None;
            }
            Some((at, String::from_utf8(bytes.to_vec()).ok()?))
        })
        .collect();
    declarations
        .iter()
        .enumerate()
        .map(|(i, (at, name))| ClassInterval {
            name: name.clone(),
            class_offset: *at,
            content: ByteRange {
                start: at + 6 + name.len(),
                end: declarations.get(i + 1).map_or(payload.len(), |n| n.0),
            },
        })
        .collect()
}
fn auxiliary_channels_are_consistent(strips: &[usize], channels: &[TessellationChannel]) -> bool {
    let [b, c, d] = channels else { return false };
    if (b.item_size, b.kind, b.flags) != (4, 8, 2)
        || (c.item_size, c.kind, c.flags) != (4, 8, 2)
        || (d.item_size, d.kind, d.flags) != (1, 8, 2)
    {
        return false;
    }
    let Some(list_c) = strips
        .iter()
        .map(|length| length.checked_mul(2)?.checked_sub(2))
        .collect::<Option<Vec<_>>>()
    else {
        return false;
    };
    let Some(endpoint_count) = list_c
        .iter()
        .try_fold(0usize, |total, count| total.checked_add(*count))
    else {
        return false;
    };
    let stored_list_c = c
        .data
        .as_chunks::<4>()
        .0
        .iter()
        .map(|bytes| usize::try_from(View::u32_le_at(bytes, 0)?).ok())
        .collect::<Option<Vec<_>>>();
    let counts = (usize::try_from(b.count).ok(), usize::try_from(d.count).ok());
    let payload_lengths = channels.iter().all(|channel| {
        usize::try_from(channel.item_size)
            .ok()
            .and_then(|size| usize::try_from(channel.count).ok()?.checked_mul(size))
            == Some(channel.data.len())
    });
    payload_lengths
        && (counts == (Some(0), Some(0)) || counts == (Some(endpoint_count), Some(endpoint_count)))
        && usize::try_from(c.count).ok() == Some(strips.len())
        && stored_list_c.as_deref() == Some(list_c.as_slice())
        && b.data
            .as_chunks::<4>()
            .0
            .iter()
            .all(|bytes| View::f32_le_at(bytes, 0).is_some_and(f32::is_finite))
}

fn parse_table(bytes: &[u8], mut at: usize) -> Option<(Mesh, usize)> {
    let mut strips = Vec::new();
    let mut vertices = Vec::new();
    let mut normals = Vec::new();
    let mut channels = Vec::new();
    for index in 0..6 {
        let item_size = View::u32_le_at(bytes, at)? as usize;
        let kind = View::u32_le_at(bytes, at + 4)?;
        let flags = View::u32_le_at(bytes, at + 8)?;
        let count = View::u32_le_at(bytes, at + 12)? as usize;
        if item_size > 16 {
            return None;
        }
        let data = at.checked_add(16)?;
        let end = data.checked_add(item_size.checked_mul(count)?)?;
        if end > bytes.len() {
            return None;
        }
        channels.push(TessellationChannel {
            item_size: item_size as u32,
            kind,
            flags,
            count: count as u32,
            data: bytes[data..end].to_vec(),
        });
        if index == 0 && item_size == 4 && kind == 8 {
            strips = (0..count)
                .map(|i| View::u32_le_at(bytes, data + i * 4).map(|v| v as usize))
                .collect::<Option<Vec<_>>>()?;
        } else if index == 1 && item_size == 12 && kind == 100 {
            for i in 0..count {
                let p = data + i * 12;
                let read = |at| {
                    View::f32_le_at(bytes, at)
                        .map(f64::from)
                        .filter(|value| value.is_finite())
                };
                vertices.push(point3(
                    read(p)? * 1000.0,
                    read(p + 4)? * 1000.0,
                    read(p + 8)? * 1000.0,
                ));
            }
        } else if index == 2 && item_size == 12 && kind == 100 {
            for i in 0..count {
                let p = data + i * 12;
                let read = |at| {
                    View::f32_le_at(bytes, at)
                        .map(f64::from)
                        .filter(|value| value.is_finite())
                };
                normals.push(point3(read(p)?, read(p + 4)?, read(p + 8)?));
            }
        }
        at = end;
    }
    let vertex_count = strips
        .iter()
        .try_fold(0usize, |total, length| total.checked_add(*length))?;
    if !matches!(channels.as_slice(), [a, positions, normals, ..]
        if (a.item_size, a.kind, a.flags) == (4, 8, 2)
            && (positions.item_size, positions.kind, positions.flags) == (12, 100, 2)
            && (normals.item_size, normals.kind, normals.flags) == (12, 100, 2))
    {
        return None;
    }
    if strips.iter().any(|n| *n < 3)
        || strips.is_empty()
        || vertices.is_empty()
        || vertex_count != vertices.len()
        || !normals.is_empty() && normals.len() != vertices.len()
        || !auxiliary_channels_are_consistent(&strips, &channels[3..])
    {
        return None;
    }
    let mut triangles = Vec::new();
    let mut base = 0usize;
    for length in &strips {
        for i in 0..length.saturating_sub(2) {
            let [a, b, c] = if i % 2 == 0 {
                [base + i, base + i + 1, base + i + 2]
            } else {
                [base + i, base + i + 2, base + i + 1]
            };
            triangles.push([
                u32::try_from(a).ok()?,
                u32::try_from(b).ok()?,
                u32::try_from(c).ok()?,
            ]);
        }
        base = base.checked_add(*length)?;
    }
    Some((
        Mesh {
            vertices,
            triangles,
            strip_lengths: strips.into_iter().map(|length| length as u32).collect(),
            normals,
        },
        at,
    ))
}

fn section_display_faces(payload: &[u8]) -> Vec<DisplayFace> {
    let classes = class_intervals(payload);
    let markers = payload
        .windows(FACE_TESSELLATION_CLASS.len())
        .enumerate()
        .filter_map(|(offset, bytes)| (bytes == FACE_TESSELLATION_CLASS).then_some(offset))
        .collect::<Vec<_>>();
    let mut faces = Vec::new();
    for (marker_index, marker) in markers.iter().copied().enumerate() {
        let header = marker + FACE_TESSELLATION_CLASS.len();
        let (Some(triangle_count), Some(strip_count)) = (
            View::u32_le_at(payload, header + compact_face::TRIANGLE_COUNT),
            View::u32_le_at(payload, header + compact_face::STRIP_COUNT),
        ) else {
            continue;
        };
        let limit = classes
            .iter()
            .find(|class| class.name == "uoTempFaceTessData_c" && class.class_offset + 6 == marker)
            .map_or_else(
                || {
                    markers
                        .get(marker_index + 1)
                        .copied()
                        .unwrap_or(payload.len())
                },
                |class| class.content.end,
            );
        let start = header + descriptor_table_offset(payload, header);
        let Some(tables) = parse_table_sequence(payload, start, limit) else {
            continue;
        };
        if !tables.first().is_some_and(|(_, _, mesh)| {
            usize::try_from(triangle_count).ok() == Some(mesh.triangles.len())
                && usize::try_from(strip_count).ok() == Some(mesh.strip_lengths.len())
        }) {
            continue;
        }
        for (start, end, mesh) in tables {
            faces.push(DisplayFace {
                mesh,
                table_index: 0,
                table: ByteRange { start, end },
                metadata: ByteRange {
                    start: end,
                    end: limit,
                },
            });
        }
    }
    append_referenced_faces(payload, &classes, &mut faces);
    faces.sort_by_key(|face| face.table.start);
    for index in 0..faces.len() {
        let metadata_end = faces
            .get(index + 1)
            .map_or(faces[index].metadata.end, |next| next.table.start)
            .min(faces[index].metadata.end);
        faces[index].table_index = index;
        faces[index].metadata.end = metadata_end;
    }
    faces
}

/// Recognize reused face classes from validated instances in the declared
/// face interval. MFC old-class tags reuse a class without repeating its name.
/// Unknown tags and bare descriptor arrays are not admitted by this path.
fn append_referenced_faces(
    payload: &[u8],
    classes: &[ClassInterval],
    faces: &mut Vec<DisplayFace>,
) {
    let mut tags = HashSet::new();
    for face in faces.iter() {
        for width in [compact_face::LEN, extended_face::LEN] {
            let Some(at) = face.table.start.checked_sub(width + 2) else {
                continue;
            };
            let Some(tag) = View::u16_le_at(payload, at) else {
                continue;
            };
            if tag & 0x8000 == 0 || tag == 0xffff || tag == 0x8000 {
                continue;
            }
            if classes.iter().any(|class| {
                class.name == "uoTempFaceTessData_c"
                    && class.content.start <= at
                    && face.table.end <= class.content.end
            }) && descriptor_table_offset(payload, at + 2) == width
                && face_header_agrees(payload, at + 2, &face.mesh)
            {
                tags.insert(tag);
            }
        }
    }
    if tags.len() != 1 {
        return;
    }
    let Some(tag) = tags.iter().next().copied() else {
        return;
    };
    let Some(first) = faces.iter().map(|face| face.table.start).min() else {
        return;
    };
    let mut ranges = faces
        .iter()
        .map(|f| (f.table.start, f.table.end))
        .collect::<Vec<_>>();
    let known = ranges.iter().copied().collect::<HashMap<_, _>>();
    ranges.sort_unstable();
    let mut ranges = ranges.into_iter().peekable();
    let mut at = first;
    while at + 2 < payload.len() {
        if let Some(&(start, end)) = ranges.peek()
            && start <= at
        {
            ranges.next();
            at = at.max(end);
            continue;
        }
        if View::u16_le_at(payload, at) != Some(tag) {
            at += 1;
            continue;
        }
        let header = at + 2;
        let start = header + descriptor_table_offset(payload, header);
        if let Some(end) = known.get(&start) {
            at = *end;
            continue;
        }
        let limit = classes
            .iter()
            .find(|class| class.class_offset > at)
            .map_or(payload.len(), |class| class.class_offset);
        if let Some((mesh, end)) = parse_table(payload, start)
            && end <= limit
            && !mesh.vertices.is_empty()
            && face_header_agrees(payload, header, &mesh)
        {
            faces.push(DisplayFace {
                mesh,
                table_index: 0,
                table: ByteRange { start, end },
                metadata: ByteRange {
                    start: end,
                    end: limit,
                },
            });
            at = end;
            continue;
        }
        at += 2;
    }
}

fn face_header_agrees(payload: &[u8], at: usize, mesh: &Mesh) -> bool {
    View::u32_le_at(payload, at + compact_face::TRIANGLE_COUNT)
        .is_some_and(|count| count as usize == mesh.triangles.len())
        && View::u32_le_at(payload, at + compact_face::STRIP_COUNT)
            .is_some_and(|count| count as usize == mesh.strip_lengths.len())
}

pub(crate) fn section_meshes(payload: &[u8]) -> Vec<(usize, Mesh)> {
    section_display_faces(payload)
        .into_iter()
        .map(|face| (face.table.start, face.mesh))
        .collect()
}

/// Offset of the first descriptor after a face-tessellation class name.
///
/// Both forms begin with two u32 cells. The extended form then carries a
/// fixed 32-byte extension. A compact table begins with item
/// size 4 at the same position, so it cannot satisfy the extension grammar.
fn descriptor_table_offset(payload: &[u8], at: usize) -> usize {
    let extended = View::u32_le_at(payload, at + extended_face::FORM) == Some(1)
        && View::u32_le_at(payload, at + extended_face::ZERO_AT_12) == Some(0)
        && View::u32_le_at(payload, at + extended_face::ZERO_AT_16) == Some(0)
        && View::u32_le_at(payload, at + extended_face::FORM_TOKEN).is_some_and(|token| token != 0)
        && payload
            .get(at + extended_face::ZERO_TAIL..at + extended_face::LEN)
            .is_some_and(|tail| tail.iter().all(|byte| *byte == 0));
    if extended {
        extended_face::LEN
    } else {
        compact_face::LEN
    }
}

fn parse_table_sequence(
    payload: &[u8],
    at: usize,
    limit: usize,
) -> Option<Vec<(usize, usize, Mesh)>> {
    let first_start = at;
    let (mesh, mut at) = parse_table(payload, at)?;
    if at > limit {
        return None;
    }
    if mesh.vertices.is_empty() {
        return None;
    }
    let mut meshes = vec![(first_start, at, mesh)];
    while at + 16 <= limit {
        let Some(relative) = payload[at..limit]
            .windows(4)
            .position(|window| window == [4, 0, 0, 0])
        else {
            break;
        };
        at += relative;
        let start = at;
        if let Some((next, end)) = parse_table(payload, at) {
            if end <= limit && !next.vertices.is_empty() {
                meshes.push((start, end, next));
                at = end;
            } else {
                at += 4;
            }
        } else {
            at += 4;
        }
    }
    Some(meshes)
}
