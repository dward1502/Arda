//! Keep SQLite's main/WAL/SHM names anchored to a directory FD for the entire
//! connection lifetime. SQLite's default Unix xFullPathname resolves proc-fd
//! aliases back into mutable host pathnames, so passing an alias alone is unsafe.
use anyhow::{bail, Result};
use rusqlite::ffi;
use std::{
    ffi::{c_char, c_int, CStr},
    sync::OnceLock,
};
const NAME: &CStr = c"arda-keeper-anchored";
static REGISTERED: OnceLock<Result<(), String>> = OnceLock::new();

unsafe extern "C" fn full_path(
    _vfs: *mut ffi::sqlite3_vfs,
    path: *const c_char,
    output_size: c_int,
    output: *mut c_char,
) -> c_int {
    if path.is_null() || output.is_null() || output_size <= 0 {
        return ffi::SQLITE_CANTOPEN;
    }
    let bytes = CStr::from_ptr(path).to_bytes();
    let Some(rest) = bytes.strip_prefix(b"/proc/self/fd/") else {
        return ffi::SQLITE_CANTOPEN;
    };
    let Some(slash) = rest.iter().position(|b| *b == b'/') else {
        return ffi::SQLITE_CANTOPEN;
    };
    if slash == 0
        || !rest[..slash].iter().all(u8::is_ascii_digit)
        || &rest[slash + 1..] != b"owner.sqlite3"
    {
        return ffi::SQLITE_CANTOPEN;
    }
    let Some(fd) = std::str::from_utf8(&rest[..slash])
        .ok()
        .and_then(|s| s.parse::<c_int>().ok())
    else {
        return ffi::SQLITE_CANTOPEN;
    };
    let mut metadata: libc::stat = std::mem::zeroed();
    if fd <= 2
        || libc::fstat(fd, &mut metadata) != 0
        || metadata.st_mode & libc::S_IFMT != libc::S_IFDIR
    {
        return ffi::SQLITE_CANTOPEN;
    }
    if bytes.len() >= output_size as usize {
        return ffi::SQLITE_CANTOPEN;
    }
    std::ptr::copy_nonoverlapping(bytes.as_ptr(), output.cast(), bytes.len());
    *output.add(bytes.len()) = 0;
    ffi::SQLITE_OK
}

pub fn register() -> Result<&'static str> {
    let result = REGISTERED.get_or_init(|| unsafe {
        let base = ffi::sqlite3_vfs_find(c"unix".as_ptr());
        if base.is_null() {
            return Err("Unix SQLite VFS unavailable".into());
        }
        // All I/O/locking remains the bundled Unix implementation; only primary
        // name canonicalization changes. The registered copy lives process-wide.
        let mut vfs = Box::new(std::ptr::read(base));
        vfs.pNext = std::ptr::null_mut();
        vfs.zName = NAME.as_ptr();
        vfs.xFullPathname = Some(full_path);
        if ffi::sqlite3_vfs_register(vfs.as_mut(), 0) != ffi::SQLITE_OK {
            return Err("anchored SQLite VFS registration failed".into());
        }
        Box::leak(vfs);
        Ok(())
    });
    if let Err(error) = result {
        bail!("{error}");
    }
    Ok("arda-keeper-anchored")
}
