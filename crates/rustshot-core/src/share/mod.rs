//! Sending a capture somewhere other than a file or the clipboard: another
//! app, the printer, an email, text recognition, an upload.
//!
//! Most of these hand the OS a file, so the image is first written to a
//! temporary PNG named like a saved capture. Temporary files are removed a
//! day later, on the next start.

pub mod ocr;
pub mod upload;

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use anyhow::{bail, Context, Result};
use image::RgbaImage;

use crate::config::{Config, OutputFormat};
use crate::filename::{self, PatternContext};
use crate::output;

/// Where temporary copies go.
fn temp_dir() -> PathBuf {
    std::env::temp_dir().join("rustshot")
}

/// Write `image` to a temporary PNG named with the filename pattern, for
/// handing to another app.
pub fn temp_copy(image: &RgbaImage, config: &Config, title: &str, time: chrono::DateTime<chrono::Local>) -> Result<PathBuf> {
    let dir = temp_dir();
    std::fs::create_dir_all(&dir).context("creating the temporary folder")?;
    let stem = filename::expand(&config.filename_pattern, &PatternContext { time, title });
    // The pattern may name subfolders; only the name matters here.
    let stem = Path::new(&stem).file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or(stem);
    let path = output::unique_path(&dir, &stem, OutputFormat::Png.extension());
    output::save_image(image, &path, OutputFormat::Png, 100)?;
    Ok(path)
}

/// Remove temporary copies older than a day. The app they were handed to
/// has long since opened them.
pub fn clean_temp() {
    let Ok(entries) = std::fs::read_dir(temp_dir()) else { return };
    let day = Duration::from_secs(24 * 60 * 60);
    for entry in entries.flatten() {
        let old = entry.metadata().and_then(|m| m.modified()).is_ok_and(|t| SystemTime::now().duration_since(t).unwrap_or_default() > day);
        if old {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

/// Open `path` in `app` (a `.app` on macOS, a program elsewhere), or in the
/// default app for images.
pub fn open_with(path: &Path, app: Option<&Path>) -> Result<()> {
    let result = match app {
        #[cfg(target_os = "macos")]
        Some(app) => std::process::Command::new("open").arg("-a").arg(app).arg(path).spawn().map(drop),
        #[cfg(not(target_os = "macos"))]
        Some(app) => std::process::Command::new(app).arg(path).spawn().map(drop),
        None => return open_default(path),
    };
    result.with_context(|| format!("couldn't start {}", app.map(|a| a.display().to_string()).unwrap_or_default()))
}

fn open_default(path: &Path) -> Result<()> {
    #[cfg(windows)]
    return shell_execute("open", path);
    #[cfg(target_os = "macos")]
    let program = "open";
    #[cfg(all(unix, not(target_os = "macos")))]
    let program = "xdg-open";
    #[cfg(not(windows))]
    std::process::Command::new(program).arg(path).spawn().map(drop).context("couldn't open the image")
}

/// Print `path` (Windows and Linux; macOS prints from the app with the
/// system print panel). Windows shows its Print Pictures window; Linux sends
/// it to the default printer.
pub fn print_file(path: &Path) -> Result<()> {
    #[cfg(windows)]
    return shell_execute("print", path);
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let status = std::process::Command::new("lp").arg(path).status().context("printing needs the `lp` command (CUPS)")?;
        if !status.success() {
            bail!("the printer refused the job ({status})");
        }
        Ok(())
    }
    #[cfg(target_os = "macos")]
    {
        let _ = path;
        bail!("printing on macOS goes through the app")
    }
}

/// Start a new email with `path` attached, in the default mail app
/// (Windows and Linux; macOS uses the app's sharing service).
pub fn email_file(path: &Path, subject: &str) -> Result<()> {
    #[cfg(windows)]
    return mapi::send(path, subject);
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let status = std::process::Command::new("xdg-email")
            .arg("--subject")
            .arg(subject)
            .arg("--attach")
            .arg(path)
            .status()
            .context("email needs the `xdg-email` command (xdg-utils)")?;
        if !status.success() {
            bail!("no mail app could start a message ({status})");
        }
        Ok(())
    }
    #[cfg(target_os = "macos")]
    {
        let _ = (path, subject);
        bail!("email on macOS goes through the app")
    }
}

#[cfg(windows)]
fn shell_execute(verb: &str, path: &Path) -> Result<()> {
    use windows::core::HSTRING;
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let result = unsafe { ShellExecuteW(None, &HSTRING::from(verb), &HSTRING::from(path.as_os_str()), None, None, SW_SHOWNORMAL) };
    // Values above 32 mean success.
    if result.0 as isize <= 32 {
        bail!("Windows couldn't {verb} the image (error {})", result.0 as isize);
    }
    Ok(())
}

/// Simple MAPI: what Windows mail apps (Outlook, Thunderbird, …) answer to
/// "send a file by email". Loaded at run time, as Windows does it.
#[cfg(windows)]
mod mapi {
    use std::path::Path;

    use anyhow::{bail, Result};
    use windows::core::{s, w, PWSTR};
    use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
    use windows::Win32::System::Mapi::{MapiFileDescW, MapiMessageW, MAPI_DIALOG, MAPI_LOGON_UI};

    /// `MAPI_USER_ABORT`: the message window was closed without sending.
    const USER_ABORT: u32 = 1;

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    pub fn send(path: &Path, subject: &str) -> Result<()> {
        unsafe {
            let Ok(lib) = LoadLibraryW(w!("mapi32.dll")) else { bail!("no email app is set up (mapi32.dll is missing)") };
            let Some(proc) = GetProcAddress(lib, s!("MAPISendMailW")) else { bail!("the email app doesn't support attachments") };
            type SendMail = unsafe extern "system" fn(usize, usize, *const MapiMessageW, u32, u32) -> u32;
            let send = std::mem::transmute::<unsafe extern "system" fn() -> isize, SendMail>(proc);

            let mut path_w = wide(&path.to_string_lossy());
            let mut name_w = wide(&path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default());
            let mut subject_w = wide(subject);
            let mut file = MapiFileDescW {
                nPosition: u32::MAX,
                lpszPathName: PWSTR(path_w.as_mut_ptr()),
                lpszFileName: PWSTR(name_w.as_mut_ptr()),
                ..Default::default()
            };
            let message = MapiMessageW { lpszSubject: PWSTR(subject_w.as_mut_ptr()), nFileCount: 1, lpFiles: &mut file, ..Default::default() };
            match send(0, 0, &message, MAPI_DIALOG | MAPI_LOGON_UI, 0) {
                0 | USER_ABORT => Ok(()),
                code => bail!("the email app couldn't start a message (MAPI error {code}). Is a default mail app set in Windows Settings?"),
            }
        }
    }
}
