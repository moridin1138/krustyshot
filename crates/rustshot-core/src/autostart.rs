//! "Open at login".
//!
//! - macOS: a LaunchAgent in `~/Library/LaunchAgents`.
//! - Windows: the `HKCU\...\Run` registry value.
//! - Linux: an XDG autostart entry in `~/.config/autostart`.

use std::path::PathBuf;

use anyhow::{Context, Result};

#[cfg(target_os = "macos")]
const LABEL: &str = "org.rustshot.Rustshot";

fn exe() -> Result<PathBuf> {
    std::env::current_exe().context("finding Rustshot's executable")
}

#[cfg(target_os = "macos")]
fn agent_path() -> Option<PathBuf> {
    directories::BaseDirs::new().map(|d| d.home_dir().join("Library/LaunchAgents").join(format!("{LABEL}.plist")))
}

/// Is Rustshot set to open at login?
pub fn is_enabled() -> bool {
    #[cfg(target_os = "macos")]
    {
        agent_path().is_some_and(|p| p.exists())
    }
    #[cfg(windows)]
    {
        std::process::Command::new("reg")
            .args(["query", r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run", "/v", "Rustshot"])
            .output()
            .is_ok_and(|o| o.status.success())
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        desktop_path().is_some_and(|p| p.exists())
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
fn desktop_path() -> Option<PathBuf> {
    directories::BaseDirs::new().map(|d| d.config_dir().join("autostart/rustshot.desktop"))
}

/// Turn "open at login" on or off.
pub fn set_enabled(on: bool) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        let path = agent_path().context("no home folder")?;
        if on {
            let exe = exe()?;
            let plist = format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>Label</key>
	<string>{LABEL}</string>
	<key>ProgramArguments</key>
	<array>
		<string>{}</string>
	</array>
	<key>RunAtLoad</key>
	<true/>
	<key>ProcessType</key>
	<string>Interactive</string>
</dict>
</plist>
"#,
                xml_escape(&exe.display().to_string())
            );
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir)?;
            }
            std::fs::write(&path, plist).with_context(|| format!("writing {}", path.display()))?;
        } else if path.exists() {
            std::fs::remove_file(&path).with_context(|| format!("removing {}", path.display()))?;
        }
        Ok(())
    }
    #[cfg(windows)]
    {
        let key = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
        let status = if on {
            let exe = exe()?;
            std::process::Command::new("reg")
                .args(["add", key, "/v", "Rustshot", "/t", "REG_SZ", "/d", &format!("\"{}\"", exe.display()), "/f"])
                .status()?
        } else {
            std::process::Command::new("reg").args(["delete", key, "/v", "Rustshot", "/f"]).status()?
        };
        anyhow::ensure!(status.success() || !on, "reg exited with {status}");
        Ok(())
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let path = desktop_path().context("no config folder")?;
        if on {
            let exe = exe()?;
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir)?;
            }
            let entry = format!(
                "[Desktop Entry]\nType=Application\nName=Rustshot\nExec=\"{}\"\nX-GNOME-Autostart-enabled=true\n",
                exe.display()
            );
            std::fs::write(&path, entry)?;
        } else if path.exists() {
            std::fs::remove_file(&path)?;
        }
        Ok(())
    }
}

#[cfg(target_os = "macos")]
fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}
