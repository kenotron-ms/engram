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

// ─── Path helpers ─────────────────────────────────────────────────────────────

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

/// Returns the `~/.engram` directory for daemon logs.
fn engram_log_dir() -> PathBuf {
    home_dir()
        .unwrap_or_else(|| {
            // $HOME not set — use /tmp as last resort; launchd/systemd don't expand ~
            PathBuf::from("/tmp")
        })
        .join(".engram")
}

/// Returns the path of the current running executable, canonicalized.
fn current_exe_path() -> String {
    let raw = std::env::current_exe()
        .unwrap_or_else(|_| PathBuf::from("engram"));
    raw.canonicalize()
        .unwrap_or(raw) // keep raw absolute path if canonicalize fails
        .to_string_lossy()
        .to_string()
}

/// Returns the current user's UID, used to construct launchd domain strings.
#[cfg(target_os = "macos")]
fn current_uid() -> u32 {
    // SAFETY: getuid(2) has no preconditions, always succeeds, and is
    // signal-safe. Documented in POSIX as always returning a valid uid_t.
    unsafe { libc::getuid() }
}

/// Returns the current `PATH` environment variable.
fn current_path_env() -> String {
    std::env::var("PATH")
        .unwrap_or_else(|_| "/usr/local/bin:/usr/bin:/bin".to_string())
}

// ─── Service content builders ─────────────────────────────────────────────────

/// Builds the launchd plist for the engram daemon service on macOS.
///
/// Uses the current executable path and `~/.engram/` for log files.
pub fn build_macos_plist() -> String {
    let exe = current_exe_path();
    let log_dir = engram_log_dir();
    let stdout = log_dir.join("daemon.log").to_string_lossy().to_string();
    let stderr = log_dir
        .join("daemon.err.log")
        .to_string_lossy()
        .to_string();
    let path_env = current_path_env();
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.engram.daemon</string>
    <key>ProgramArguments</key>
    <array>
        <string>{exe}</string>
        <string>daemon</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>EnvironmentVariables</key>
    <dict>
        <key>PATH</key>
        <string>{path_env}</string>
    </dict>
    <key>StandardOutPath</key>
    <string>{stdout}</string>
    <key>StandardErrorPath</key>
    <string>{stderr}</string>
</dict>
</plist>"#
    )
}

/// Builds the systemd user service unit for the engram daemon on Linux.
///
/// Uses the current executable path and the current `PATH` environment variable.
// Always compiled so that unit tests can validate the content on all platforms.
#[allow(dead_code)]
pub fn build_linux_unit() -> String {
    let exe = current_exe_path();
    let path_env = current_path_env();
    format!(
        "[Unit]\nDescription=engram memory daemon\nAfter=network.target\n\n[Service]\nExecStart={exe} daemon\nRestart=on-failure\nRestartSec=5\nEnvironment=PATH={path_env}\n\n[Install]\nWantedBy=default.target\n"
    )
}

// ─── Service install ───────────────────────────────────────────────────────────

/// Install the engram daemon as a platform-managed background service.
///
/// - **macOS**: writes `~/Library/LaunchAgents/com.engram.daemon.plist` and
///   runs `launchctl bootstrap gui/<uid>`.
/// - **Linux**: writes `~/.config/systemd/user/engram.service` and runs
///   `systemctl --user enable` then `systemctl --user start`.
/// - **Windows**: prints a "coming soon" notice.
pub fn install_service() -> Result<(), InstallError> {
    #[cfg(target_os = "macos")]
    {
        let dir = launchagents_dir().ok_or(InstallError::UnsupportedPlatform)?;
        std::fs::create_dir_all(&dir)?;
        let plist_path = dir.join("com.engram.daemon.plist");
        std::fs::write(&plist_path, build_macos_plist())?;

        let uid = current_uid();
        let domain = format!("gui/{uid}");
        let status = std::process::Command::new("launchctl")
            .args(["bootstrap", &domain, &plist_path.to_string_lossy()])
            .status()?;

        if !status.success() {
            return Err(InstallError::Command(format!(
                "launchctl bootstrap exited with status {}",
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
        std::fs::write(&service_path, build_linux_unit())?;

        let status = std::process::Command::new("systemctl")
            .args(["--user", "enable", "engram.service"])
            .status()?;

        if !status.success() {
            return Err(InstallError::Command(format!(
                "systemctl --user enable exited with status {}",
                status
            )));
        }

        let status = std::process::Command::new("systemctl")
            .args(["--user", "start", "engram.service"])
            .status()?;

        if !status.success() {
            return Err(InstallError::Command(format!(
                "systemctl --user start exited with status {}",
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

// ─── Service uninstall ────────────────────────────────────────────────────────

/// Uninstall the engram daemon background service.
///
/// - **macOS**: runs `launchctl bootout gui/<uid>` and removes the plist.
/// - **Linux**: runs `systemctl --user disable` then `systemctl --user stop`
///   and removes the unit file.
/// - **Windows**: prints a "coming soon" notice.
pub fn uninstall_service() -> Result<(), InstallError> {
    #[cfg(target_os = "macos")]
    {
        let dir = launchagents_dir().ok_or(InstallError::UnsupportedPlatform)?;
        let plist_path = dir.join("com.engram.daemon.plist");

        // Best-effort bootout (ignore errors if service was never loaded).
        let uid = current_uid();
        let domain = format!("gui/{uid}");
        let _ = std::process::Command::new("launchctl")
            .args(["bootout", &domain, &plist_path.to_string_lossy()])
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

        // Best-effort disable and stop.
        let _ = std::process::Command::new("systemctl")
            .args(["--user", "disable", "engram.service"])
            .status();

        let _ = std::process::Command::new("systemctl")
            .args(["--user", "stop", "engram.service"])
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

    #[test]
    fn plist_contains_current_exe_path() {
        let plist = build_macos_plist();
        let exe = std::env::current_exe()
            .unwrap()
            .canonicalize()
            .unwrap_or_else(|_| std::env::current_exe().unwrap())
            .to_string_lossy()
            .to_string();
        assert!(
            plist.contains(&exe),
            "plist should contain current exe path {exe}, got:\n{plist}"
        );
    }

    #[test]
    fn plist_logs_to_engram_dir_not_tmp() {
        let plist = build_macos_plist();
        assert!(!plist.contains("/tmp/"), "plist should not log to /tmp/");
        assert!(plist.contains(".engram"), "plist should log to ~/.engram/");
    }

    #[test]
    fn systemd_unit_contains_current_exe_path() {
        let unit = build_linux_unit();
        let exe = std::env::current_exe()
            .unwrap()
            .canonicalize()
            .unwrap_or_else(|_| std::env::current_exe().unwrap())
            .to_string_lossy()
            .to_string();
        assert!(
            unit.contains(&exe),
            "systemd unit should contain current exe path {exe}"
        );
    }

    /// The macOS plist must contain all required structural keys.
    #[test]
    fn test_macos_plist_contains_required_keys() {
        let plist = build_macos_plist();
        assert!(
            plist.contains("com.engram.daemon"),
            "plist must contain the label 'com.engram.daemon'"
        );
        assert!(
            plist.contains("ProgramArguments"),
            "plist must contain 'ProgramArguments'"
        );
        assert!(
            plist.contains("<string>daemon</string>"),
            "plist ProgramArguments must include 'daemon'"
        );
        assert!(
            plist.contains("RunAtLoad"),
            "plist must contain 'RunAtLoad'"
        );
        assert!(
            plist.contains("KeepAlive"),
            "plist must contain 'KeepAlive'"
        );
    }

    /// The Linux systemd service must contain all required fields.
    #[test]
    fn test_linux_service_contains_required_fields() {
        let unit = build_linux_unit();
        assert!(
            unit.contains("daemon"),
            "unit must reference 'daemon' subcommand in ExecStart"
        );
        assert!(
            unit.contains("Restart=on-failure"),
            "unit must contain 'Restart=on-failure'"
        );
        assert!(
            unit.contains("RestartSec=5"),
            "unit must contain 'RestartSec=5'"
        );
        assert!(
            unit.contains("WantedBy=default.target"),
            "unit must contain 'WantedBy=default.target'"
        );
    }
}
