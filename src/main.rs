mod cli;
mod detect;
mod classify;
mod ops;
mod prompt;

use anyhow::{Context, Result};
use clap::Parser;
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use std::path::PathBuf;

use crate::cli::Args;
use crate::detect::{is_binary, resolve_extension};
use crate::classify::Category;
use crate::ops::move_to_category;
use crate::prompt::{BinaryAction, BinaryPolicy};

struct ProcessingResult {
    moved: Vec<(String, String)>,
    skipped: Vec<String>,
    warnings: Vec<String>,
}

impl ProcessingResult {
    fn new() -> Self {
        Self {
            moved: Vec::new(),
            skipped: Vec::new(),
            warnings: Vec::new(),
        }
    }
}

fn print_banner() {
    let version = env!("CARGO_PKG_VERSION");
    println!(
        "{} v{}\n{}",
        "[ Sortify ]".bright_cyan().bold(),
        version,
        "→ A lightweight utility for organizing files\n---------------------------------------------"
            .dimmed()
    );
}

fn is_self_binary(entry: &PathBuf, exe: &Option<PathBuf>) -> bool {
    exe.as_ref().is_some_and(|p| p == entry)
}

fn collect_files(cwd: &PathBuf) -> Result<Vec<PathBuf>> {
    let entries: Vec<_> = fs::read_dir(cwd)
        .context("cannot read current directory")?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .collect();

    Ok(entries)
}

fn create_progress_bar(total: u64) -> ProgressBar {
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::with_template("{spinner:.green} {msg:.bold.dimmed} [{pos}/{len}]")
            .unwrap()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ "),
    );
    pb
}

fn process_file(
    entry: PathBuf,
    cwd: &PathBuf,
    current_exe: &Option<PathBuf>,
    policy: &mut BinaryPolicy,
    args: &Args,
    result: &mut ProcessingResult,
) -> Result<()> {
    let canonical = fs::canonicalize(&entry).unwrap_or_else(|_| entry.clone());

    if is_self_binary(&canonical, current_exe) {
        result.skipped.push(entry.display().to_string());
        return Ok(());
    }

    let res = resolve_extension(&entry, args.ext_only, args.dry_run)?;
    let ext_opt = res.ext;
    
    if let Some((sig, real)) = res.mismatch {
        result.warnings.push(format!(
            "Signature/ext mismatch: {} (sig: .{}, ext: .{})",
            entry.display(),
            sig,
            real
        ));
    }

    let ext = match ext_opt {
        Some(e) => e,
        None => {
            result.skipped.push(entry.display().to_string());
            return Ok(());
        }
    };

    if !args.ext_only && is_binary(&entry)? {
        if args.dry_run {
            result.warnings.push(format!("Binary file detected: {}", entry.display()));
            result.skipped.push(entry.display().to_string());
            return Ok(());
        }

        let (action, new_policy) = policy.decide(&entry)?;
        *policy = new_policy;

        if let BinaryAction::Skip = action {
            result.skipped.push(entry.display().to_string());
            return Ok(());
        }
    }

    let category = Category::from_ext(&ext);
    move_to_category(&entry, cwd, &category, args.dry_run)
        .with_context(|| format!("failed to move {}", entry.display()))?;

    result.moved.push((entry.display().to_string(), category.dir_name().to_string()));
    Ok(())
}

fn print_results(result: &ProcessingResult, is_dry_run: bool) {
    println!("{}", "Sorting completed".green().bold());
    println!();

    if is_dry_run {
        println!("{}", "Dry run summary:".cyan().bold());
    } else {
        println!("{}", "Moved files:".green().bold());
    }

    if result.moved.is_empty() {
        println!("  (none)");
    } else {
        for (src, dest) in &result.moved {
            println!("  {} {} {}", src.dimmed(), "→".bright_black(), dest.bold());
        }
    }

    if !result.skipped.is_empty() {
        println!("\n{}", "Skipped:".yellow().bold());
        for name in &result.skipped {
            println!("  {}", name.dimmed());
        }
    }

    if !result.warnings.is_empty() {
        println!("\n{}", "Dry-run warnings:".bright_yellow().bold());
        for warn in &result.warnings {
            println!("  {}", warn.dimmed());
        }
    }
}

fn print_summary(result: &ProcessingResult, is_dry_run: bool) {
    println!("\nSummary:");
    if is_dry_run {
        println!(
            "  {} {}",
            "Would move:".cyan(),
            result.moved.len().to_string().bold()
        );
        println!(
            "  {} {}",
            "Would skip:".cyan(),
            result.skipped.len().to_string().bold()
        );
        println!(
            "  {} {}",
            "Warnings:".yellow(),
            result.warnings.len().to_string().bold()
        );
    } else {
        println!("  {} {}", "Moved:".green(), result.moved.len().to_string().bold());
        println!(
            "  {} {}",
            "Skipped:".yellow(),
            result.skipped.len().to_string().bold()
        );
    }
    println!();
}

fn main() -> Result<()> {
    print_banner();
    let args = Args::parse();

    let cwd = std::env::current_dir().context("cannot get current directory")?;
    let current_exe = std::env::current_exe().ok().and_then(|p| fs::canonicalize(p).ok());
    
    let entries = collect_files(&cwd)?;

    if entries.is_empty() {
        println!("{}", "No files found in current directory.".dimmed());
        return Ok(());
    }

    println!("{}", "\nProcessing files...".bold());

    let pb = create_progress_bar(entries.len() as u64);
    let mut policy = BinaryPolicy::AskEvery;
    let mut result = ProcessingResult::new();

    for entry in entries {
        let filename = entry.file_name().and_then(|s| s.to_str()).unwrap_or("unknown");
        pb.set_message(format!("Processing {}", filename));
        pb.tick();

        process_file(entry, &cwd, &current_exe, &mut policy, &args, &mut result)?;
        pb.inc(1);
    }

    pb.finish_and_clear();

    print_results(&result, args.dry_run);
    print_summary(&result, args.dry_run);

    Ok(())
}