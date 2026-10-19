use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// Sizes are in decimal megabytes (1 MB = 1,000,000 bytes), which is slightly
/// stricter than the binary MiB some services use — a conservative choice.
const MB: u64 = 1_000_000;

pub const TELEGRAM_MAX_FILE_SIZE: u64 = 50 * MB;
pub const DISCORD_MAX_FILE_SIZE: u64 = 10 * MB;
pub const NTFY_MAX_FILE_SIZE: u64 = 15 * MB;

/// A file the user asked to send, validated up front.
/// The bytes are only read when a backend actually uploads it.
#[derive(Debug, Clone)]
pub struct Attachment {
    pub path: PathBuf,
    pub name: String,
    pub size: u64,
}

impl Attachment {
    /// Check that the path is a readable regular file and record its name and size.
    pub fn open(path: &Path) -> Result<Self> {
        let meta = std::fs::metadata(path)
            .with_context(|| format!("cannot access file {}", path.display()))?;
        if !meta.is_file() {
            anyhow::bail!("{} is not a regular file", path.display());
        }
        // Opening proves we have read permission.
        std::fs::File::open(path)
            .with_context(|| format!("cannot read file {}", path.display()))?;

        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "file".to_string());

        Ok(Self {
            path: path.to_path_buf(),
            name,
            size: meta.len(),
        })
    }

    pub fn read(&self) -> Result<Vec<u8>> {
        std::fs::read(&self.path)
            .with_context(|| format!("cannot read file {}", self.path.display()))
    }
}

/// Fail if any attachment exceeds `limit` bytes.
pub fn check_sizes(attachments: &[Attachment], limit: u64) -> Result<()> {
    for att in attachments {
        if att.size > limit {
            anyhow::bail!(
                "file {} is {}, over the {} limit",
                att.name,
                format_size(att.size),
                format_size(limit)
            );
        }
    }
    Ok(())
}

pub fn format_size(bytes: u64) -> String {
    if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= 1000 {
        format!("{:.1} KB", bytes as f64 / 1000.0)
    } else {
        format!("{bytes} B")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn att(size: u64) -> Attachment {
        Attachment {
            path: PathBuf::from("/tmp/x.bin"),
            name: "x.bin".to_string(),
            size,
        }
    }

    #[test]
    fn under_and_at_limit_ok() {
        assert!(check_sizes(&[att(1), att(DISCORD_MAX_FILE_SIZE)], DISCORD_MAX_FILE_SIZE).is_ok());
    }

    #[test]
    fn over_limit_fails_with_size_and_limit() {
        let err = check_sizes(&[att(1), att(12_300_000)], DISCORD_MAX_FILE_SIZE).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("x.bin"), "{msg}");
        assert!(msg.contains("12.3 MB"), "{msg}");
        assert!(msg.contains("10.0 MB"), "{msg}");
    }

    #[test]
    fn limits_match_backends() {
        assert_eq!(TELEGRAM_MAX_FILE_SIZE, 50_000_000);
        assert_eq!(DISCORD_MAX_FILE_SIZE, 10_000_000);
        assert_eq!(NTFY_MAX_FILE_SIZE, 15_000_000);
    }

    #[test]
    fn format_size_units() {
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(1_500), "1.5 KB");
        assert_eq!(format_size(50_000_000), "50.0 MB");
    }

    #[test]
    fn open_rejects_missing_file() {
        let err = Attachment::open(Path::new("/definitely/not/here.txt")).unwrap_err();
        assert!(err.to_string().contains("cannot access file"));
    }

    #[test]
    fn open_rejects_directory() {
        let dir = std::env::temp_dir();
        let err = Attachment::open(&dir).unwrap_err();
        assert!(err.to_string().contains("not a regular file"));
    }

    #[test]
    fn open_reads_name_and_size() {
        let path =
            std::env::temp_dir().join(format!("pygmy-attach-test-{}.txt", std::process::id()));
        std::fs::write(&path, b"hello").unwrap();
        let a = Attachment::open(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        assert_eq!(a.size, 5);
        assert!(a.name.starts_with("pygmy-attach-test-"));
    }
}
