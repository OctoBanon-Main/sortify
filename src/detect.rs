use anyhow::{Context, Result};
use std::fs;
use std::io::Read;
use std::path::Path;

use crate::prompt::{ask_conflict_resolution, ConflictResolution};

const HEADER_CAP: usize = 64;

struct Sig {
    pattern: &'static [u8],
    offset: usize,
    ext: &'static str,
}

const FIXED_SIGNATURES: &[Sig] = &[
    Sig { pattern: b"\x89PNG\r\n\x1A\n", offset: 0, ext: "png" },
    Sig { pattern: b"\xFF\xD8\xFF", offset: 0, ext: "jpg" },
    Sig { pattern: b"GIF87a", offset: 0, ext: "gif" },
    Sig { pattern: b"GIF89a", offset: 0, ext: "gif" },
    Sig { pattern: b"BM", offset: 0, ext: "bmp" },
    Sig { pattern: b"%PDF", offset: 0, ext: "pdf" },
    Sig { pattern: b"%!PS-Adobe-", offset: 0, ext: "ps" },
    Sig { pattern: b"PK\x03\x04", offset: 0, ext: "zip" },
    Sig { pattern: b"\x1F\x8B\x08", offset: 0, ext: "gz" },
    Sig { pattern: b"\x1A\x45\xDF\xA3", offset: 0, ext: "mkv" },
    Sig { pattern: b"WEBP", offset: 8, ext: "webp" },
    Sig { pattern: b"ID3", offset: 0, ext: "mp3" },
    Sig { pattern: b"OggS", offset: 0, ext: "ogg" },
    Sig { pattern: b"fLaC", offset: 0, ext: "flac" },
    Sig { pattern: b"\x00\x00\x01\x00", offset: 0, ext: "ico" },
    Sig { pattern: b"II*\x00", offset: 0, ext: "tif" },
    Sig { pattern: b"MM\x00*", offset: 0, ext: "tif" },
    Sig { pattern: b"Rar!\x1A\x07\x00", offset: 0, ext: "rar" },
    Sig { pattern: b"7z\xBC\xAF\x27\x1C", offset: 0, ext: "7z" },
];

const BINARY_SIGNATURES: &[Sig] = &[
    Sig { pattern: b"MZ", offset: 0, ext: "exe" },
    Sig { pattern: b"\x7FELF", offset: 0, ext: "elf" },
    Sig { pattern: b"\xCA\xFE\xBA\xBE", offset: 0, ext: "mach-o" },
    Sig { pattern: b"\xCF\xFA\xED\xFE", offset: 0, ext: "mach-o" },
    Sig { pattern: b"\xFE\xED\xFA\xCF", offset: 0, ext: "mach-o" },
    Sig { pattern: b"\xFE\xED\xFA\xCE", offset: 0, ext: "mach-o" },
    Sig { pattern: b"\x00asm", offset: 0, ext: "wasm" },
];

fn read_prefix(path: &Path, cap: usize) -> Result<Vec<u8>> {
    let mut f = fs::File::open(path)
        .with_context(|| format!("cannot open file to read header: {}", path.display()))?;

    let file_size = f
        .metadata()
        .with_context(|| format!("cannot stat file: {}", path.display()))?
        .len() as usize;
    let read_size = cap.min(file_size);

    let mut buf = vec![0u8; read_size];
    let n = f
        .read(&mut buf)
        .with_context(|| format!("cannot read header from {}", path.display()))?;
    buf.truncate(n);
    Ok(buf)
}

fn starts_with_at(buf: &[u8], offset: usize, pat: &[u8]) -> bool {
    buf.len() >= offset + pat.len() && &buf[offset..offset + pat.len()] == pat
}

fn contains(buf: &[u8], pat: &[u8]) -> bool {
    if pat.is_empty() || buf.len() < pat.len() {
        return false;
    }
    buf.windows(pat.len()).any(|w| w == pat)
}

fn detect_mp4_like(buf: &[u8]) -> Option<&'static str> {
    if buf.len() < 12 || !starts_with_at(buf, 4, b"ftyp") {
        return None;
    }
    Some(match &buf[8..12] {
        b"isom" | b"iso2" | b"mp41" | b"mp42" | b"avc1" | b"MSNV" | b"mp71" => "mp4",
        b"M4V " => "m4v",
        b"M4A " => "m4a",
        b"M4B " => "m4b",
        b"qt  " => "mov",
        _ => "mp4",
    })
}

fn detect_riff_typed(buf: &[u8]) -> Option<&'static str> {
    if buf.len() < 12 || !starts_with_at(buf, 0, b"RIFF") {
        return None;
    }

    match &buf[8..12] {
        b"WEBP" => Some("webp"),
        b"WAVE" => Some("wav"),
        b"AVI " => Some("avi"),
        _ => None,
    }
}

fn detect_zip_like(buf: &[u8]) -> Option<&'static str> {
    if !starts_with_at(buf, 0, b"PK\x03\x04") {
        return None;
    }

    let slice = buf;

    if contains(slice, b"[Content_Types].xml") || contains(slice, b"word/") {
        return Some("docx");
    }
    if contains(slice, b"xl/") {
        return Some("xlsx");
    }
    if contains(slice, b"ppt/") {
        return Some("pptx");
    }
    if contains(slice, b"AndroidManifest.xml") {
        return Some("apk");
    }
    if contains(slice, b"META-INF/") {
        return Some("jar");
    }

    Some("zip")
}

fn detect_json(buf: &[u8]) -> Option<&'static str> {
    let start = if buf.starts_with(b"\xEF\xBB\xBF") { 3 } else { 0 };

    let first_char = buf[start..]
        .iter()
        .copied()
        .find(|b| !b.is_ascii_whitespace())?;

    if first_char == b'{' || first_char == b'[' {
        let has_json_chars = buf[start..]
            .iter()
            .any(|&b| b == b'"' || b == b':' || b == b',');

        if has_json_chars {
            return Some("json");
        }
    }

    None
}

fn detect_fixed(buf: &[u8]) -> Option<&'static str> {
    FIXED_SIGNATURES
        .iter()
        .find(|sig| starts_with_at(buf, sig.offset, sig.pattern))
        .map(|sig| sig.ext)
}

fn looks_binary(buf: &[u8]) -> bool {
    if buf.is_empty() {
        return false;
    }

    if buf.iter().any(|&b| b == 0) {
        return true;
    }

    let non_text = buf
        .iter()
        .filter(|&&b| !matches!(b, 0x09 | 0x0A | 0x0D | 0x20..=0x7E))
        .count();

    (non_text as f32) / (buf.len() as f32) > 0.30
}

fn detect_by_signature_buf(buf: &[u8]) -> Option<&'static str> {
    if buf.is_empty() {
        return None;
    }

    if let Some(ext) = detect_mp4_like(buf) {
        return Some(ext);
    }
    if let Some(ext) = detect_riff_typed(buf) {
        return Some(ext);
    }
    if let Some(ext) = detect_zip_like(buf) {
        return Some(ext);
    }
    if let Some(ext) = detect_json(buf) {
        return Some(ext);
    }

    detect_fixed(buf)
}

fn detect_by_signature(path: &Path) -> Result<Option<&'static str>> {
    let buf = read_prefix(path, HEADER_CAP)?;
    Ok(detect_by_signature_buf(&buf))
}

pub fn is_binary(path: &Path) -> Result<bool> {
    let buf = read_prefix(path, HEADER_CAP)?;
    if buf.is_empty() {
        return Ok(false);
    }

    if BINARY_SIGNATURES
        .iter()
        .any(|sig| starts_with_at(&buf, sig.offset, sig.pattern))
    {
        return Ok(true);
    }

    if detect_mp4_like(&buf).is_some()
        || detect_riff_typed(&buf).is_some()
        || detect_zip_like(&buf).is_some()
        || matches!(
            detect_fixed(&buf),
            Some(
                "png"
                    | "jpg"
                    | "gif"
                    | "bmp"
                    | "pdf"
                    | "ps"
                    | "webp"
                    | "mkv"
                    | "ico"
                    | "tif"
                    | "gz"
                    | "rar"
                    | "7z"
                    | "mp3"
                    | "ogg"
                    | "flac"
                    | "zip"
            )
        )
    {
        return Ok(true);
    }

    if detect_json(&buf).is_some() {
        return Ok(false);
    }

    Ok(looks_binary(&buf))
}

fn ext_from_path(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase())
}

#[derive(Debug)]
pub struct ResolveResult {
    pub ext: Option<String>,
    pub mismatch: Option<(String, String)>,
}

pub fn resolve_extension(path: &Path, ext_only: bool, dry_run: bool) -> Result<ResolveResult> {
    if ext_only {
        let ext = ext_from_path(path).unwrap_or_else(|| "unknown".to_string());
        return Ok(ResolveResult {
            ext: Some(ext),
            mismatch: None,
        });
    }

    if let Some(sig_ext) = detect_by_signature(path)? {
        let actual_ext = ext_from_path(path);

        if let Some(actual) = actual_ext.as_deref() {
            if actual != sig_ext {
                if dry_run {
                    return Ok(ResolveResult {
                        ext: Some(sig_ext.to_string()),
                        mismatch: Some((sig_ext.to_string(), actual.to_string())),
                    });
                } else {
                    match ask_conflict_resolution(path, sig_ext, actual)? {
                        ConflictResolution::Skip => {
                            return Ok(ResolveResult {
                                ext: None,
                                mismatch: None,
                            })
                        }
                        ConflictResolution::BySignature(chosen) => {
                            return Ok(ResolveResult {
                                ext: Some(chosen),
                                mismatch: None,
                            })
                        }
                        ConflictResolution::ByExtension(chosen) => {
                            return Ok(ResolveResult {
                                ext: Some(chosen),
                                mismatch: None,
                            })
                        }
                        ConflictResolution::Mismatched => {
                            return Ok(ResolveResult {
                                ext: Some("mismatch".to_string()),
                                mismatch: Some((sig_ext.to_string(), actual.to_string())),
                            })
                        }
                    }
                }
            }
        }

        return Ok(ResolveResult {
            ext: Some(sig_ext.to_string()),
            mismatch: None,
        });
    }

    Ok(ResolveResult {
        ext: Some(ext_from_path(path).unwrap_or_else(|| "unknown".to_string())),
        mismatch: None,
    })
}