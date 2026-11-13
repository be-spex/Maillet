//! Opens files and links with the desktop's default program.

use std::ffi::OsStr;
use std::process::Command;

/// Starts `xdg-open` for `target` and returns at once. A thread waits for it,
/// so the finished process does not stay behind as a zombie.
pub fn xdg_open(target: impl AsRef<OsStr>) -> std::io::Result<()> {
    let mut child = Command::new("xdg-open").arg(target).spawn()?;
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}
