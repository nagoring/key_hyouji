use std::io;
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use windows::core::{Interface, HSTRING, PCWSTR};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, IPersistFile, CLSCTX_INPROC_SERVER,
    COINIT_APARTMENTTHREADED,
};
use windows::Win32::UI::Shell::{IShellLinkW, SHGetKnownFolderPath, ShellLink, FOLDERID_Startup, KF_FLAG_DEFAULT};

fn to_io(e: windows::core::Error) -> io::Error {
    io::Error::other(e.to_string())
}

fn startup_dir() -> io::Result<PathBuf> {
    unsafe {
        let p = SHGetKnownFolderPath(&FOLDERID_Startup, KF_FLAG_DEFAULT, None).map_err(to_io)?;
        let s = p.to_string().map_err(|e| io::Error::other(e.to_string()));
        CoTaskMemFree(Some(p.0 as *const _));
        Ok(PathBuf::from(s?))
    }
}

fn link_path() -> io::Result<PathBuf> {
    Ok(startup_dir()?.join("key_hyouji.lnk"))
}

fn wide(p: &Path) -> Vec<u16> {
    p.as_os_str().encode_wide().chain(std::iter::once(0)).collect()
}

fn create_link(link: &Path, exe: &Path) -> io::Result<()> {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let shell: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).map_err(to_io)?;
        let exe_w = wide(exe);
        shell.SetPath(PCWSTR(exe_w.as_ptr())).map_err(to_io)?;
        if let Some(dir) = exe.parent() {
            let dir_w = wide(dir);
            shell.SetWorkingDirectory(PCWSTR(dir_w.as_ptr())).map_err(to_io)?;
        }
        shell.SetDescription(&HSTRING::from("key_hyouji")).map_err(to_io)?;
        let file: IPersistFile = shell.cast().map_err(to_io)?;
        let link_w = wide(link);
        file.Save(PCWSTR(link_w.as_ptr()), true).map_err(to_io)
    }
}

pub fn set_enabled(enabled: bool, exe: &Path) -> io::Result<()> {
    let link = link_path()?;
    if enabled {
        create_link(&link, exe)
    } else {
        match std::fs::remove_file(&link) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            r => r,
        }
    }
}

pub fn is_enabled() -> bool {
    link_path().map(|p| p.exists()).unwrap_or(false)
}
