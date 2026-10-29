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
    std::env::current_exe().context("finding Krustyshot's executable")
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

/// The login entry as stored (plist, registry query output or desktop file),
/// to see which executable it starts.
fn recorded() -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        std::fs::read_to_string(agent_path()?).ok()
    }
    #[cfg(windows)]
    {
        let out = std::process::Command::new("reg")
            .args(["query", r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run", "/v", "Rustshot"])
            .output()
            .ok()?;
        out.status.success().then(|| String::from_utf8_lossy(&out.stdout).into_owned())
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::fs::read_to_string(desktop_path()?).ok()
    }
}

/// If "open at login" is on but starts another copy (such as the
/// `rustshot` executable from before the rename to Krustyshot), point it at
/// this one. Development builds (inside a `target` folder) leave it alone.
pub fn refresh() -> Result<()> {
    let exe = exe()?;
    if !is_enabled() || exe.components().any(|c| c.as_os_str() == "target") {
        return Ok(());
    }
    if recorded().is_some_and(|r| !r.contains(&exe.display().to_string())) {
        log::info!("open at login: now starts {}", exe.display());
        set_enabled(true)?;
    }
    Ok(())
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
                "[Desktop Entry]\nType=Application\nName=Krustyshot\nExec=\"{}\"\nX-GNOME-Autostart-enabled=true\n",
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
