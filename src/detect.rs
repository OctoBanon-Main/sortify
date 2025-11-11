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
    Sig { pattern: b"PK\x03\x04", offset: 0, ext: "zip" },
    Sig { pattern: b"\x1F\x8B\x08", offset: 0, ext: "gz" },
    Sig { pattern: b"\x1A\x45\xDF\xA3", offset: 0, ext: "mkv" },
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
    
    let file_size = f.metadata()?.len() as usize;
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

/// MP4/QuickTime и прочие ISO BMFF
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

/// RIFF-контейнеры
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

/// Улучшенная детекция JSON
fn detect_json(buf: &[u8]) -> Option<&'static str> {
    // Пропускаем BOM если есть
    let start = if buf.starts_with(b"\xEF\xBB\xBF") { 3 } else { 0 };
    
    let first_char = buf[start..]
        .iter()
        .copied()
        .find(|b| !b.is_ascii_whitespace())?;

    // Проверяем только явные JSON-маркеры
    if first_char == b'{' || first_char == b'[' {
        // Дополнительная проверка: должны быть кавычки или запятые (типично для JSON)
        let has_json_chars = buf[start..]
            .iter()
            .any(|&b| b == b'"' || b == b':' || b == b',');
        
        if has_json_chars {
            return Some("json");
        }
    }
    
    None
}

/// Просто пройтись по фиксированным сигнатурам
fn detect_fixed(buf: &[u8]) -> Option<&'static str> {
    FIXED_SIGNATURES
        .iter()
        .find(|sig| starts_with_at(buf, sig.offset, sig.pattern))
        .map(|sig| sig.ext)
}

/// Общее определение по сигнатуре из буфера
fn detect_by_signature_buf(buf: &[u8]) -> Option<&'static str> {
    // Если файл слишком маленький, не пытаемся детектировать
    if buf.is_empty() {
        return None;
    }

    if let Some(ext) = detect_mp4_like(buf) {
        return Some(ext);
    }
    if let Some(ext) = detect_riff_typed(buf) {
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
    let is_bin = BINARY_SIGNATURES
        .iter()
        .any(|sig| starts_with_at(&buf, sig.offset, sig.pattern));
    Ok(is_bin)
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
        return Ok(ResolveResult { ext: Some(ext), mismatch: None });
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
                            return Ok(ResolveResult { ext: None, mismatch: None })
                        }
                        ConflictResolution::BySignature(chosen) => {
                            return Ok(ResolveResult { ext: Some(chosen), mismatch: None })
                        }
                        ConflictResolution::ByExtension(chosen) => {
                            return Ok(ResolveResult { ext: Some(chosen), mismatch: None })
                        }
                        ConflictResolution::Mismatched => {
                            return Ok(ResolveResult {
                                ext: Some("mismatch".to_string()),
                                mismatch: None,
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