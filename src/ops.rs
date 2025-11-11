use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

use crate::classify::Category;

fn get_unique_path(target: &Path) -> PathBuf {
    if !target.exists() {
        return target.to_path_buf();
    }

    let parent = target.parent().unwrap();
    let stem = target.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
    let ext = target.extension().and_then(|e| e.to_str());

    for i in 1..10000 {
        let new_name = if let Some(extension) = ext {
            format!("{}_{}.{}", stem, i, extension)
        } else {
            format!("{}_{}", stem, i)
        };

        let new_path = parent.join(new_name);
        if !new_path.exists() {
            return new_path;
        }
    }

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    let fallback_name = if let Some(extension) = ext {
        format!("{}_{}.{}", stem, timestamp, extension)
    } else {
        format!("{}_{}", stem, timestamp)
    };
    
    parent.join(fallback_name)
}

pub fn move_to_category(
    src: &Path,
    root: &Path,
    category: &Category,
    dry_run: bool,
) -> Result<()> {
    let target_dir = root.join(category.dir_name());
    let file_name = src.file_name().context("file has no name")?;
    let mut target_path = target_dir.join(file_name);

    if dry_run {
        return Ok(());
    }

    fs::create_dir_all(&target_dir)
        .with_context(|| format!("cannot create dir {}", target_dir.display()))?;

    if target_path.exists() {
        eprintln!(
            "File already exists: {}",
            target_path.display()
        );
        target_path = get_unique_path(&target_path);
        eprintln!("   Renaming to: {}", target_path.file_name().unwrap().to_string_lossy());
    }

    fs::rename(src, &target_path)
        .with_context(|| format!("cannot move {} to {}", src.display(), target_path.display()))?;

    Ok(())
}