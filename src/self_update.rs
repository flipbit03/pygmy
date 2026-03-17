use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use colored::Colorize;

use crate::version_check;

#[derive(Debug, Parser)]
pub struct SelfCmd {
    #[command(subcommand)]
    pub action: SelfAction,
}

#[derive(Debug, Subcommand)]
pub enum SelfAction {
    /// Update pygmy to the latest release.
    Update(UpdateArgs),
}

#[derive(Debug, Parser)]
pub struct UpdateArgs {
    /// Just check if an update is available, don't install it.
    #[arg(long)]
    check: bool,
}

pub async fn run(cmd: SelfCmd) -> Result<()> {
    match cmd.action {
        SelfAction::Update(args) => run_update(args).await,
    }
}

async fn run_update(args: UpdateArgs) -> Result<()> {
    if version_check::is_dev_build() {
        eprintln!("Running a dev build (0.0.0) — self-update is not supported.");
        return Ok(());
    }

    let current = version_check::current_version();
    let latest = version_check::get_latest_version(true)
        .await
        .context("could not determine the latest version")?;

    if args.check {
        if !version_check::is_newer(current, &latest) {
            println!("pygmy {} is already the latest version.", current.green());
        } else {
            println!(
                "Update available: {} → {}",
                current.yellow(),
                latest.green()
            );
            println!("Run `pygmy self update` to upgrade.");
        }
        return Ok(());
    }

    if !version_check::is_newer(current, &latest) {
        println!("pygmy {} is already the latest version.", current.green());
        return Ok(());
    }

    println!("Updating pygmy {} → {}", current.yellow(), latest.green());

    if cfg!(feature = "binary-release") {
        update_binary(&latest).await
    } else {
        update_cargo().await
    }
}

async fn update_binary(version: &str) -> Result<()> {
    let url = version_check::release_asset_url(version)?;
    let current_exe =
        std::env::current_exe().context("could not determine current executable path")?;

    println!("Downloading from GitHub Releases...");

    let client = reqwest::Client::new();
    let bytes = client
        .get(&url)
        .header("User-Agent", "pygmy-self-update")
        .send()
        .await
        .context("failed to download release binary")?
        .error_for_status()
        .context("download failed")?
        .bytes()
        .await
        .context("failed to read response body")?;

    let tmp_path = current_exe.with_extension("tmp-update");
    std::fs::write(&tmp_path, &bytes).context("failed to write temporary file")?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o755);
        std::fs::set_permissions(&tmp_path, perms)
            .context("failed to set executable permissions")?;
    }

    std::fs::rename(&tmp_path, &current_exe)
        .context("failed to replace binary — you may need to run with appropriate permissions")?;

    println!(
        "pygmy {} installed to {}",
        version.green(),
        current_exe.display()
    );
    Ok(())
}

async fn update_cargo() -> Result<()> {
    let cargo_check = std::process::Command::new("cargo")
        .arg("--version")
        .output();
    if cargo_check.is_err() || !cargo_check.unwrap().status.success() {
        anyhow::bail!(
            "pygmy was installed via cargo, but `cargo` is not in your PATH.\n\
             Install Rust from https://rustup.rs or add cargo to your PATH."
        );
    }

    println!("Running `cargo install pygmy`...");

    let status = std::process::Command::new("cargo")
        .args(["install", "pygmy"])
        .status()
        .context("failed to run `cargo install pygmy`")?;

    if !status.success() {
        anyhow::bail!("`cargo install pygmy` exited with status {}", status);
    }

    println!("{}", "Update complete.".green());
    Ok(())
}
