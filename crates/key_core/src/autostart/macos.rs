use std::fs;
use std::io;
use std::path::{Path, PathBuf};

fn plist_path() -> io::Result<PathBuf> {
    let home = std::env::var_os("HOME").ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "HOME is not set"))?;
    Ok(PathBuf::from(home).join("Library/LaunchAgents").join(format!("{}.plist", super::LABEL)))
}

pub fn set_enabled(enabled: bool, exe: &Path) -> io::Result<()> {
    let path = plist_path()?;
    if enabled {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        fs::write(&path, super::launch_agent_plist(super::LABEL, &exe.to_string_lossy()))
    } else {
        match fs::remove_file(&path) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            r => r,
        }
    }
}

pub fn is_enabled() -> bool {
    plist_path().map(|p| p.exists()).unwrap_or(false)
}
