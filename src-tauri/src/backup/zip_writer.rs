use anyhow::{bail, Context, Result};
use std::path::Path;

// ── Public API ────────────────────────────────────────────────────────

/// Pack a directory tree into a ZIP archive held in memory.
///
/// Uses the *stored* (method 0) method: no compression. A DepEd SF2 `.xls` is
/// an OLE2 compound file whose streams are already deflated, so deflating it
/// again buys little and costs a dependency. The archive is still a valid ZIP,
/// which is what matters for the single-object model Google Drive imposes on
/// an upload.
///
/// Entries are emitted in sorted order so the same tree always produces the
/// same bytes.
pub(crate) fn zip_directory_to_bytes(source_dir: &Path) -> Result<Vec<u8>> {
    if !source_dir.is_dir() {
        bail!("cannot archive {}: not a directory", source_dir.display());
    }

    let mut files = Vec::new();
    collect_files(source_dir, source_dir, &mut files)?;
    files.sort_by(|left, right| left.name.cmp(&right.name));

    let mut writer = ZipWriter::default();
    for file in &files {
        let bytes = std::fs::read(&file.path)
            .with_context(|| format!("failed to read {}", file.path.display()))?;
        writer.add(&file.name, &bytes)?;
    }
    Ok(writer.finish())
}

// ── Collection ────────────────────────────────────────────────────────

struct ArchiveEntry {
    /// Slash-separated path relative to the archived root.
    name: String,
    path: std::path::PathBuf,
}

fn collect_files(root: &Path, dir: &Path, out: &mut Vec<ArchiveEntry>) -> Result<()> {
    let entries = std::fs::read_dir(dir)
        .with_context(|| format!("failed to read {}", dir.display()))?
        .collect::<std::result::Result<Vec<_>, _>>()
        .with_context(|| format!("failed to read entries in {}", dir.display()))?;

    for entry in entries {
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            collect_files(root, &path, out)?;
            continue;
        }
        if !entry.file_type()?.is_file() {
            continue;
        }
        let relative = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .components()
            .map(|component| component.as_os_str().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("/");
        out.push(ArchiveEntry {
            name: relative,
            path,
        });
    }
    Ok(())
}

// ── ZIP Writer (stored entries only) ──────────────────────────────────

const LOCAL_FILE_HEADER_SIGNATURE: u32 = 0x0403_4b50;
const CENTRAL_DIRECTORY_SIGNATURE: u32 = 0x0201_4b50;
const END_OF_CENTRAL_DIRECTORY_SIGNATURE: u32 = 0x0605_4b50;
const VERSION_NEEDED: u16 = 20;
const METHOD_STORED: u16 = 0;
/// Bit 11 — the file name is UTF-8.
const FLAG_UTF8_NAMES: u16 = 0x0800;

struct PendingEntry {
    name: Vec<u8>,
    crc32: u32,
    size: u32,
    local_header_offset: u32,
}

#[derive(Default)]
struct ZipWriter {
    body: Vec<u8>,
    entries: Vec<PendingEntry>,
}

impl ZipWriter {
    fn add(&mut self, name: &str, bytes: &[u8]) -> Result<()> {
        let size = u32::try_from(bytes.len())
            .with_context(|| format!("{name} is too large for a ZIP archive"))?;
        let offset = u32::try_from(self.body.len())
            .context("backup folder is too large for a ZIP archive")?;
        let name_bytes = name.as_bytes().to_vec();

        self.body
            .extend_from_slice(&LOCAL_FILE_HEADER_SIGNATURE.to_le_bytes());
        self.body.extend_from_slice(&VERSION_NEEDED.to_le_bytes());
        self.body
            .extend_from_slice(&(FLAG_UTF8_NAMES | METHOD_STORED).to_le_bytes());
        self.body.extend_from_slice(&0u16.to_le_bytes()); // last-mod time
        self.body.extend_from_slice(&0u16.to_le_bytes()); // last-mod date
        self.body.extend_from_slice(&crc32(bytes).to_le_bytes());
        self.body.extend_from_slice(&size.to_le_bytes()); // compressed size
        self.body.extend_from_slice(&size.to_le_bytes()); // uncompressed size
        self.body
            .extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
        self.body.extend_from_slice(&0u16.to_le_bytes()); // extra field length
        self.body.extend_from_slice(&name_bytes);
        self.body.extend_from_slice(bytes);

        self.entries.push(PendingEntry {
            name: name_bytes,
            crc32: crc32(bytes),
            size,
            local_header_offset: offset,
        });
        Ok(())
    }

    fn finish(mut self) -> Vec<u8> {
        let central_directory_offset = self.body.len() as u32;
        for entry in &self.entries {
            self.body
                .extend_from_slice(&CENTRAL_DIRECTORY_SIGNATURE.to_le_bytes());
            self.body.extend_from_slice(&VERSION_NEEDED.to_le_bytes());
            self.body.extend_from_slice(&VERSION_NEEDED.to_le_bytes());
            self.body
                .extend_from_slice(&(FLAG_UTF8_NAMES | METHOD_STORED).to_le_bytes());
            self.body.extend_from_slice(&0u16.to_le_bytes()); // last-mod time
            self.body.extend_from_slice(&0u16.to_le_bytes()); // last-mod date
            self.body.extend_from_slice(&entry.crc32.to_le_bytes());
            self.body.extend_from_slice(&entry.size.to_le_bytes());
            self.body.extend_from_slice(&entry.size.to_le_bytes());
            self.body
                .extend_from_slice(&(entry.name.len() as u16).to_le_bytes());
            self.body.extend_from_slice(&0u16.to_le_bytes()); // extra field length
            self.body.extend_from_slice(&0u16.to_le_bytes()); // comment length
            self.body.extend_from_slice(&0u16.to_le_bytes()); // disk number
            self.body.extend_from_slice(&0u16.to_le_bytes()); // internal attrs
            self.body.extend_from_slice(&0u32.to_le_bytes()); // external attrs
            self.body
                .extend_from_slice(&entry.local_header_offset.to_le_bytes());
            self.body.extend_from_slice(&entry.name);
        }
        let central_directory_size = self.body.len() as u32 - central_directory_offset;

        self.body
            .extend_from_slice(&END_OF_CENTRAL_DIRECTORY_SIGNATURE.to_le_bytes());
        self.body.extend_from_slice(&0u16.to_le_bytes()); // this disk
        self.body.extend_from_slice(&0u16.to_le_bytes()); // disk with CD
        self.body
            .extend_from_slice(&(self.entries.len() as u16).to_le_bytes());
        self.body
            .extend_from_slice(&(self.entries.len() as u16).to_le_bytes());
        self.body
            .extend_from_slice(&central_directory_size.to_le_bytes());
        self.body
            .extend_from_slice(&central_directory_offset.to_le_bytes());
        self.body.extend_from_slice(&0u16.to_le_bytes()); // comment length
        self.body
    }
}

/// CRC-32 (IEEE 802.3), computed without a table so the ZIP writer stays
/// self-contained.
fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFF_u32;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            let mask = if crc & 1 == 1 { 0xEDB8_8320 } else { 0 };
            crc = (crc >> 1) ^ mask;
        }
    }
    !crc
}
