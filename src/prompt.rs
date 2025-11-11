use anyhow::{Context, Result};
use colored::*;
use dialoguer::{Select, theme::Theme};
use std::path::Path;

#[derive(Default)]
struct PlainTheme;

impl Theme for PlainTheme {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryAction {
    Skip,
    Process,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryPolicy {
    AskEvery,
    SkipAll,
    NeverSkip,
}

impl BinaryPolicy {
    pub fn decide(self, file: &Path) -> Result<(BinaryAction, BinaryPolicy)> {
        match self {
            BinaryPolicy::AskEvery => ask_binary_policy_once(file),
            BinaryPolicy::SkipAll => {
                eprintln!(
                    "{} {}",
                    "Skipped binary file:".yellow().bold(),
                    file.display()
                );
                Ok((BinaryAction::Skip, BinaryPolicy::SkipAll))
            }
            BinaryPolicy::NeverSkip => Ok((BinaryAction::Process, BinaryPolicy::NeverSkip)),
        }
    }
}

pub fn ask_binary_policy_once(file: &Path) -> Result<(BinaryAction, BinaryPolicy)> {
    println!(
        "\n{} {}",
        "Binary file detected:".bright_yellow().bold(),
        file.display()
    );

    let options = &[
        "Skip this file",
        "Skip all binary files",
        "Process this file (ask again next time)",
        "Always process binary files without asking",
    ];

    let choice = Select::with_theme(&PlainTheme::default())
        .with_prompt("Choose an option")
        .items(options)
        .default(0)
        .interact()
        .context("failed to read user input")?;

    let result = match choice {
        0 => {
            println!("{}", "This binary file will be skipped once.".dimmed());
            (BinaryAction::Skip, BinaryPolicy::AskEvery)
        }
        1 => {
            println!("{}", "All binary files will be skipped automatically.".dimmed());
            (BinaryAction::Skip, BinaryPolicy::SkipAll)
        }
        2 => {
            println!("{}", "This binary file will be processed (will ask next time).".dimmed());
            (BinaryAction::Process, BinaryPolicy::AskEvery)
        }
        3 => {
            println!("{}", "All binary files will be processed automatically.".dimmed());
            (BinaryAction::Process, BinaryPolicy::NeverSkip)
        }
        _ => unreachable!(),
    };

    Ok(result)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConflictResolution {
    Skip,
    BySignature(String),
    ByExtension(String),
    Mismatched,
}

pub fn ask_conflict_resolution(
    file: &Path,
    sig_ext: &str,
    real_ext: &str,
) -> Result<ConflictResolution> {
    println!(
        "\n{}",
        "Detected mismatch between extension and file signature:".bright_red().bold()
    );
    println!("File: {}", file.display());
    println!("Declared extension: .{}", real_ext.cyan());
    println!("Detected signature: .{}", sig_ext.cyan());

    let options = &[
        "Skip this file",
        &format!("Use signature type (.{})", sig_ext),
        &format!("Use declared extension (.{})", real_ext),
        "Move to manual verification folder",
    ];

    let choice = Select::with_theme(&PlainTheme::default())
        .with_prompt("Choose an option")
        .items(options)
        .default(0)
        .interact()
        .context("failed to read user input")?;

    let res = match choice {
        0 => {
            println!("{}", "File skipped.".dimmed());
            ConflictResolution::Skip
        }
        1 => {
            println!(
                "{} .{}",
                "File will be sorted based on signature".green(),
                sig_ext.bold()
            );
            ConflictResolution::BySignature(sig_ext.to_string())
        }
        2 => {
            println!(
                "{} .{}",
                "File will be sorted based on extension".green(),
                real_ext.bold()
            );
            ConflictResolution::ByExtension(real_ext.to_string())
        }
        3 => {
            println!("{}", "File will be moved to manual verification folder.".dimmed());
            ConflictResolution::Mismatched
        }
        _ => unreachable!(),
    };

    Ok(res)
}