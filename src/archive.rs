// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! ZIP/TAR imports without extracting files to disk.
use crate::ParseError;
use std::{
    collections::BTreeMap,
    io::{Cursor, Read},
    sync::Arc,
};

/// Input and expanded-output limits for browser archive imports.
#[derive(Clone, Copy, Debug)]
pub struct ArchiveLimits {
    /// Maximum uploaded archive bytes.
    pub input_bytes: usize,
    /// Maximum expanded bytes in an individual member.
    pub member_bytes: usize,
    /// Maximum sum of expanded member sizes, including ignored non-CAD files.
    pub total_bytes: usize,
    /// Maximum archive entries, including directories and ignored files.
    pub entries: usize,
}
impl ArchiveLimits {
    /// Remove policy caps for caller-owned local imports. Available memory and
    /// platform address space still bound imports; integrity checks remain active.
    pub const fn unlimited() -> Self {
        Self {
            input_bytes: usize::MAX,
            member_bytes: usize::MAX,
            total_bytes: usize::MAX,
            entries: usize::MAX,
        }
    }
}
/// Extraction progress after a fully validated archive entry.
#[derive(Clone, Copy, Debug)]
pub struct ArchiveProgress<'a> {
    /// Validated entries, including directories and ignored members.
    pub completed_entries: usize,
    /// ZIP entry count; TAR does not declare a total in its header.
    pub total_entries: Option<usize>,
    /// Original member path, presented as text rather than a URL.
    pub path: &'a str,
}
impl Default for ArchiveLimits {
    fn default() -> Self {
        Self {
            input_bytes: 128 << 20,
            member_bytes: 64 << 20,
            total_bytes: 256 << 20,
            entries: 4096,
        }
    }
}
/// Native documents retained from an uploaded archive, addressed by virtual paths.
#[derive(Debug, Default)]
pub struct FileSet {
    /// Normalized relative archive paths and original CAD bytes.
    pub documents: BTreeMap<String, Arc<[u8]>>,
    /// Number of non-native regular members skipped without retaining their bytes.
    pub ignored_files: usize,
}
fn error(s: impl Into<String>) -> ParseError {
    ParseError(s.into())
}
fn native(path: &str) -> bool {
    let p = path.to_ascii_lowercase();
    [".sldprt", ".sldasm", ".slddrw"]
        .iter()
        .any(|s| p.ends_with(s))
}
/// Normalize an untrusted member path; absolute paths, parent traversal and
/// Windows drive/alternate-stream syntax are rejected rather than rewritten.
pub fn normalize_path(path: &str) -> Result<String, ParseError> {
    let path = path.replace('\\', "/");
    if path.starts_with('/') || path.contains(':') || path.contains('\0') || path.len() > 4096 {
        return Err(error("unsafe archive member path"));
    }
    let mut parts = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => return Err(error("archive member traverses outside its root")),
            s => parts.push(s),
        }
    }
    if parts.is_empty() {
        return Err(error("empty archive member path"));
    }
    Ok(parts.join("/"))
}
/// Result of resolving a saved CAD reference against the caller's virtual files.
#[derive(Debug, PartialEq, Eq)]
pub enum Resolution {
    /// Unique normalized file-set path; no host filesystem was consulted.
    Found(String),
    /// No matching document is present in this upload.
    Missing,
    /// Several equally plausible source paths remain; callers must not pick one.
    Ambiguous(Vec<String>),
}
impl FileSet {
    /// Read ZIP (stored/deflate), raw TAR, or gzip-compressed TAR into memory.
    /// Links, duplicate case-insensitive paths, unsafe names and over-budget
    /// archives fail atomically; an old viewer session need not be discarded.
    pub fn from_archive(
        bytes: &[u8],
        filename: &str,
        limits: ArchiveLimits,
    ) -> Result<Self, ParseError> {
        Self::from_archive_with_progress(bytes, filename, limits, |_| {})
    }
    /// Import with progress notifications after each validated entry. ZIP has a
    /// declared entry total; TAR reports completed entries without a percentage.
    /// A final notification does not imply success until integrity checks finish.
    pub fn from_archive_with_progress(
        bytes: &[u8],
        filename: &str,
        limits: ArchiveLimits,
        mut progress: impl FnMut(ArchiveProgress<'_>),
    ) -> Result<Self, ParseError> {
        if bytes.len() > limits.input_bytes {
            return Err(error("archive exceeds upload byte limit"));
        }
        let mut loader = Loader {
            set: Self::default(),
            limits,
            names: Default::default(),
            total: 0,
            count: 0,
        };
        if bytes.starts_with(b"PK") {
            let mut archive = zip::ZipArchive::new(Cursor::new(bytes))
                .map_err(|e| error(format!("invalid ZIP: {e}")))?;
            if archive.len() > limits.entries {
                return Err(error("archive entry count limit exceeded"));
            }
            let total_entries = archive.len();
            for i in 0..total_entries {
                let mut member = archive
                    .by_index(i)
                    .map_err(|e| error(format!("unsupported or damaged ZIP member: {e}")))?;
                if member.unix_mode().is_some_and(|m| m & 0o170000 == 0o120000) {
                    return Err(error("archive links are unsupported"));
                }
                let name = member.name().to_owned();
                let size = member.size();
                let dir = member.is_dir();
                loader.add(&name, size, dir, &mut member)?;
                progress(ArchiveProgress {
                    completed_entries: loader.count,
                    total_entries: Some(total_entries),
                    path: &name,
                });
            }
        } else if filename.to_ascii_lowercase().ends_with(".tar.gz")
            || filename.to_ascii_lowercase().ends_with(".tgz")
        {
            let mut expanded = Vec::new();
            flate2::read::MultiGzDecoder::new(bytes)
                .take((limits.total_bytes as u64).saturating_add(1))
                .read_to_end(&mut expanded)
                .map_err(|e| error(format!("invalid gzip: {e}")))?;
            if expanded.len() > limits.total_bytes {
                return Err(error("expanded TAR byte limit exceeded"));
            }
            loader.tar(&expanded, &mut progress)?;
        } else if filename.to_ascii_lowercase().ends_with(".tar") {
            loader.tar(bytes, &mut progress)?;
        } else {
            return Err(error("supported archives: .zip, .tar and .tar.gz (.tgz)"));
        }
        if loader.set.documents.is_empty() {
            return Err(error("archive contains no native SolidWorks documents"));
        }
        Ok(loader.set)
    }
    /// Resolve owner-relative paths first, then the longest unique suffix of a
    /// saved Windows path. Ambiguous basenames remain explicit.
    pub fn resolve(&self, owner: &str, reference: &str) -> Resolution {
        let reference = reference.replace('\\', "/");
        let parts: Vec<_> = reference
            .split('/')
            .filter(|p| !p.is_empty() && *p != ".")
            .collect();
        if parts.is_empty() {
            return Resolution::Missing;
        }
        if !reference.starts_with('/') && !reference.contains(':') {
            let mut relative: Vec<_> = owner.split('/').collect();
            relative.pop();
            let mut escaped = false;
            for part in &parts {
                if *part == ".." {
                    if relative.pop().is_none() {
                        escaped = true;
                        break;
                    }
                } else {
                    relative.push(part);
                }
            }
            if escaped {
                return Resolution::Missing;
            }
            let path = relative.join("/").to_lowercase();
            if let Some(found) = self.documents.keys().find(|p| p.to_lowercase() == path) {
                return Resolution::Found(found.clone());
            }
        }
        let mut best = 0;
        let mut candidates = Vec::new();
        for path in self.documents.keys() {
            let score = path
                .split('/')
                .rev()
                .zip(parts.iter().rev())
                .take_while(|(a, b)| a.to_lowercase() == b.to_lowercase())
                .count();
            if score == 0 {
                continue;
            }
            if score > best {
                best = score;
                candidates.clear();
            }
            if score == best {
                candidates.push(path.clone());
            }
        }
        match candidates.len() {
            0 => Resolution::Missing,
            1 => Resolution::Found(candidates.remove(0)),
            _ => Resolution::Ambiguous(candidates),
        }
    }
}
struct Loader {
    set: FileSet,
    limits: ArchiveLimits,
    names: BTreeMap<String, String>,
    total: u64,
    count: usize,
}
impl Loader {
    fn add(
        &mut self,
        path: &str,
        size: u64,
        dir: bool,
        reader: &mut impl Read,
    ) -> Result<(), ParseError> {
        self.count += 1;
        if self.count > self.limits.entries {
            return Err(error("archive entry count limit exceeded"));
        }
        if dir && matches!(path, "." | "./" | "") {
            return Ok(());
        }
        let name = normalize_path(path)?;
        if self
            .names
            .insert(name.to_lowercase(), name.clone())
            .is_some()
        {
            return Err(error("duplicate case-insensitive archive path"));
        }
        if dir {
            return Ok(());
        }
        self.total = self
            .total
            .checked_add(size)
            .ok_or_else(|| error("archive size overflow"))?;
        if size > self.limits.member_bytes as u64 || self.total > self.limits.total_bytes as u64 {
            return Err(error("expanded member/archive byte limit exceeded"));
        }
        // Read even ignored files: ZIP CRC and actual output length must validate.
        let mut bytes = Vec::new();
        reader
            .take(size.saturating_add(1))
            .read_to_end(&mut bytes)
            .map_err(|e| error(format!("damaged archive member: {e}")))?;
        if bytes.len() as u64 != size {
            return Err(error("archive member size mismatch"));
        }
        if native(&name) {
            self.set.documents.insert(name, bytes.into());
        } else {
            self.set.ignored_files += 1;
        }
        Ok(())
    }
    fn tar(
        &mut self,
        bytes: &[u8],
        progress: &mut impl FnMut(ArchiveProgress<'_>),
    ) -> Result<(), ParseError> {
        let mut archive = tar::Archive::new(Cursor::new(bytes));
        for member in archive
            .entries()
            .map_err(|e| error(format!("invalid TAR: {e}")))?
        {
            let mut member = member.map_err(|e| error(format!("invalid TAR member: {e}")))?;
            let kind = member.header().entry_type();
            if !kind.is_file() && !kind.is_dir() {
                return Err(error("TAR links and special members are unsupported"));
            }
            let name = std::str::from_utf8(&member.path_bytes())
                .map_err(|_| error("non-UTF8 TAR member path"))?
                .to_owned();
            let size = member.size();
            self.add(&name, size, kind.is_dir(), &mut member)?;
            progress(ArchiveProgress {
                completed_entries: self.count,
                total_entries: None,
                path: &name,
            });
        }
        Ok(())
    }
}
