use anyhow::Result;
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::blocking::Client;
use reqwest::header::USER_AGENT;
use semver::Version;
use serde::Deserialize;

use crate::updater::platform_check::target_suffix;

const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Deserialize)]
pub struct UpdateRelease {
    pub tag_name: String,
    pub assets: Vec<UpdateAsset>,
}

#[derive(Deserialize)]
pub struct UpdateAsset {
    pub name: String,
    pub browser_download_url: String,
}

fn find_asset_url(release: &UpdateRelease) -> Option<String> {
    let suffix = target_suffix();
    release
        .assets
        .iter()
        .find(|a| a.name.ends_with(&suffix))
        .map(|a| a.browser_download_url.clone())
}

pub fn check_for_updates() -> Result<Option<UpdateRelease>> {
    println!(
        "{}",
        "[ Sortify Updater ]".bright_cyan().bold()
    );
    println!("{}", "→ Checking for updates...".dimmed());

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} {msg}")
            .unwrap()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ "),
    );
    pb.enable_steady_tick(std::time::Duration::from_millis(100));
    pb.set_message("Contacting GitHub...");

    let client = Client::new();
    let response = client
        .get("https://api.github.com/repos/OctoBanon-Main/sortify/releases/latest")
        .header(USER_AGENT, "sortify-updater")
        .send();

    let release: UpdateRelease = match response {
        Ok(r) => {
            let result = r.error_for_status()?.json()?;
            pb.finish_and_clear();
            result
        }
        Err(err) => {
            pb.finish_and_clear();
            eprintln!("{}", format!("Failed to check updates: {}", err).red());
            return Ok(None);
        }
    };

    let latest = Version::parse(release.tag_name.trim_start_matches('v'))?;
    let current = Version::parse(CURRENT_VERSION)?;

    if latest <= current {
        println!("{}", format!("You're using the latest version (v{})", current).green());
        println!();
        return Ok(None);
    }

    println!("{}", "Update available!".yellow());
    println!("  Current version: v{}", current);
    println!("  Latest version:  v{}", latest);

    if let Some(url) = find_asset_url(&release) {
        println!("  Download: {}", url);
        println!();
        println!("{}", "Tip: Run \"sortify --update\" to install the new version.".dimmed());
    } else {
        println!("  No suitable asset found for this platform.");
    }

    println!();
    Ok(Some(release))
}