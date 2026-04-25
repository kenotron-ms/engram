// install.rs — engram install, engram uninstall, engram doctor helpers

use directories::UserDirs;
use std::io;
use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur during install/uninstall operations.
#[derive(Debug, Error)]
pub enum InstallError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Command error: {0}")]
    Command(String),

    #[error("Unsupported platform")]
    UnsupportedPlatform,
}

// ─── macOS launchd plist ───────────────────────────────────────────────────────

/// launchd plist for the engram daemon service on macOS.
pub const MACOS_PLIST: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
    "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.engram.daemon</string>
    <key>ProgramArguments</key>
    <array>
        <string>/usr/local/bin/engram</string>
        <string>daemon</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>/tmp/engram-daemon.log</string>
    <key>StandardErrorPath</key>
    <string>/tmp/engram-daemon.err.log</string>
</dict>
</plist>
"#;

// ─── Linux systemd unit ────────────────────────────────────────────────────────

/// systemd user service unit for the engram daemon on Linux.
// Always compiled so that unit tests can validate the constant on all platforms.
#[allow(dead_code)]
pub const LINUX_SERVICE: &str = "[Unit]\n\
Description=engram memory daemon\n\
After=network.target\n\
\n\
[Service]\n\
ExecStart=%h/.cargo/bin/engram daemon\n\
Restart=on-failure\n\
RestartSec=5\n\
\n\
[Install]\n\
WantedBy=default.target\n";

// ─── Path helpers ──────────────────────────────────────────────────────────────

/// Returns the current user's home directory via `directories::UserDirs`.
pub fn home_dir() -> Option<PathBuf> {
    UserDirs::new().map(|u| u.home_dir().to_path_buf())
}

/// Returns `~/Library/LaunchAgents` on macOS.
#[cfg(target_os = "macos")]
pub fn launchagents_dir() -> Option<PathBuf> {
    home_dir().map(|h| h.join("Library/LaunchAgents"))
}

/// Returns `~/.config/systemd/user` on Linux.
#[cfg(target_os = "linux")]
pub fn systemd_user_dir() -> Option<PathBuf> {
    home_dir().map(|h| h.join(".config/systemd/user"))
}

// ─── Service install ───────────────────────────────────────────────────────────

/// Install the engram daemon as a platform-managed background service.
///
/// - **macOS**: writes `~/Library/LaunchAgents/com.engram.daemon.plist` and
///   runs `launchctl load`.
/// - **Linux**: writes `~/.config/systemd/user/engram.service` and runs
///   `systemctl --user enable engram.service`.
/// - **Windows**: prints a "coming soon" notice.
pub fn install_service() -> Result<(), InstallError> {
    #[cfg(target_os = "macos")]
    {
        let dir = launchagents_dir().ok_or(InstallError::UnsupportedPlatform)?;
        std::fs::create_dir_all(&dir)?;
        let plist_path = dir.join("com.engram.daemon.plist");
        std::fs::write(&plist_path, MACOS_PLIST)?;

        let status = std::process::Command::new("launchctl")
            .args(["load", plist_path.to_str().unwrap_or("")])
            .status()?;

        if !status.success() {
            return Err(InstallError::Command(format!(
                "launchctl load exited with status {}",
                status
            )));
        }
        return Ok(());
    }

    #[cfg(target_os = "linux")]
    {
        let dir = systemd_user_dir().ok_or(InstallError::UnsupportedPlatform)?;
        std::fs::create_dir_all(&dir)?;
        let service_path = dir.join("engram.service");
        std::fs::write(&service_path, LINUX_SERVICE)?;

        let status = std::process::Command::new("systemctl")
            .args(["--user", "enable", "engram.service"])
            .status()?;

        if !status.success() {
            return Err(InstallError::Command(format!(
                "systemctl --user enable exited with status {}",
                status
            )));
        }
        return Ok(());
    }

    #[cfg(target_os = "windows")]
    {
        println!("Service installation coming soon on Windows.");
        return Ok(());
    }

    // Fallback for unsupported platforms (should not be reached on the
    // platforms above because each branch returns).
    #[allow(unreachable_code)]
    Err(InstallError::UnsupportedPlatform)
}

// ─── Service uninstall ─────────────────────────────────────────────────────────

/// Uninstall the engram daemon background service.
///
/// - **macOS**: runs `launchctl unload` and removes the plist.
/// - **Linux**: runs `systemctl --user disable` and removes the unit file.
/// - **Windows**: prints a "coming soon" notice.
pub fn uninstall_service() -> Result<(), InstallError> {
    #[cfg(target_os = "macos")]
    {
        let dir = launchagents_dir().ok_or(InstallError::UnsupportedPlatform)?;
        let plist_path = dir.join("com.engram.daemon.plist");

        // Best-effort unload (ignore errors if service was never loaded).
        let _ = std::process::Command::new("launchctl")
            .args(["unload", plist_path.to_str().unwrap_or("")])
            .status();

        if plist_path.exists() {
            std::fs::remove_file(&plist_path)?;
        }
        return Ok(());
    }

    #[cfg(target_os = "linux")]
    {
        let dir = systemd_user_dir().ok_or(InstallError::UnsupportedPlatform)?;
        let service_path = dir.join("engram.service");

        // Best-effort disable.
        let _ = std::process::Command::new("systemctl")
            .args(["--user", "disable", "engram.service"])
            .status();

        if service_path.exists() {
            std::fs::remove_file(&service_path)?;
        }
        return Ok(());
    }

    #[cfg(target_os = "windows")]
    {
        println!("Service uninstallation coming soon on Windows.");
        return Ok(());
    }

    #[allow(unreachable_code)]
    Err(InstallError::UnsupportedPlatform)
}

// ─── Unit tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// The macOS plist must contain all required keys and values.
    #[test]
    fn test_macos_plist_contains_required_keys() {
        assert!(
            MACOS_PLIST.contains("com.engram.daemon"),
            "MACOS_PLIST must contain the label 'com.engram.daemon'"
        );
        assert!(
            MACOS_PLIST.contains("ProgramArguments"),
            "MACOS_PLIST must contain 'ProgramArguments'"
        );
        assert!(
            MACOS_PLIST.contains("/usr/local/bin/engram"),
            "MACOS_PLIST must contain '/usr/local/bin/engram'"
        );
        assert!(
            MACOS_PLIST.contains("<string>daemon</string>"),
            "MACOS_PLIST ProgramArguments must include 'daemon'"
        );
        assert!(
            MACOS_PLIST.contains("RunAtLoad"),
            "MACOS_PLIST must contain 'RunAtLoad'"
        );
        assert!(
            MACOS_PLIST.contains("KeepAlive"),
            "MACOS_PLIST must contain 'KeepAlive'"
        );
        assert!(
            MACOS_PLIST.contains("/tmp/engram-daemon"),
            "MACOS_PLIST must reference stdout/stderr log paths under /tmp/engram-daemon*"
        );
    }

    /// The Linux systemd service must contain all required fields.
    #[test]
    fn test_linux_service_contains_required_fields() {
        assert!(
            LINUX_SERVICE.contains("ExecStart=%h/.cargo/bin/engram daemon"),
            "LINUX_SERVICE must contain 'ExecStart=%h/.cargo/bin/engram daemon'"
        );
        assert!(
            LINUX_SERVICE.contains("Restart=on-failure"),
            "LINUX_SERVICE must contain 'Restart=on-failure'"
        );
        assert!(
            LINUX_SERVICE.contains("RestartSec=5"),
            "LINUX_SERVICE must contain 'RestartSec=5'"
        );
        assert!(
            LINUX_SERVICE.contains("WantedBy=default.target"),
            "LINUX_SERVICE must contain 'WantedBy=default.target'"
        );
    }
}
