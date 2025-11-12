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
    pub prerelease: bool,
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

pub fn check_for_updates(include_prerelease: bool) -> Result<Option<UpdateRelease>> {
    println!("{}", "[ Sortify Updater ]".bright_cyan().bold());
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

    let url = [
        "https://api.github.com/repos/OctoBanon-Main/sortify/releases",
        "https://api.github.com/repos/OctoBanon-Main/sortify/releases/latest",
    ]
    .iter()
    .find(|_| include_prerelease)
    .unwrap_or(&"https://api.github.com/repos/OctoBanon-Main/sortify/releases/latest");

    let response = client
        .get(*url)
        .header(USER_AGENT, "sortify-updater")
        .send();

    let parsed = response
        .ok()
        .and_then(|r| r.error_for_status().ok())
        .map(|resp| {
            if include_prerelease {
                resp.json::<Vec<UpdateRelease>>().ok().and_then(|mut releases| {
                    releases
                        .drain(..)
                        .filter(|r| include_prerelease || !r.prerelease)
                        .next()
                })
            } else {
                resp.json::<UpdateRelease>().ok()
            }
        })
        .flatten();

    pb.finish_and_clear();

    parsed
        .map(handle_release)
        .transpose()
        .map(|opt| opt.flatten())
        .or_else(|err| {
            eprintln!("{}", format!("Failed to check updates: {}", err).red());
            Ok(None)
        })
}

fn handle_release(release: UpdateRelease) -> Result<Option<UpdateRelease>> {
    let latest = Version::parse(release.tag_name.trim_start_matches('v'))?;
    let current = Version::parse(CURRENT_VERSION)?;

    match latest.cmp(&current) {
        std::cmp::Ordering::Less | std::cmp::Ordering::Equal => {
            println!(
                "{}",
                format!("You're using the latest version (v{})", current).green()
            );
            println!();
            Ok(None)
        }
        std::cmp::Ordering::Greater => {
            let label = if release.prerelease {
                "Pre-release update available!".yellow()
            } else {
                "Update available!".yellow()
            };

            println!("{}", label);
            println!("  Current version: v{}", current);
            println!("  Latest version:  v{}", latest);

            find_asset_url(&release)
                .map(|url| {
                    println!("  Download: {}", url);
                    println!();
                    println!("{}", "Tip: Run \"sortify --update\" to install the new version.".dimmed());
                })
                .unwrap_or_else(|| println!("  No suitable asset found for this platform."));

            println!();
            Ok(Some(release))
        }
    }
}