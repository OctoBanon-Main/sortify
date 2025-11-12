use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version)]
pub struct Args {
    /// Disable signature detection and sort files by extension only (like the legacy FileSorter).
    #[arg(long)]
    pub ext_only: bool,

    /// Dry run (do not actually move any files)
    #[arg(long)]
    pub dry_run: bool,

    /// Skip checking for updates on startup
    #[arg(long)]
    pub no_check_updates: bool,

    /// Enable the pre-release update channel
    #[arg(long)]
    pub prerelease_channel: bool,
}