//! Inode flags (attributes) manipulations.

use std::{
    fs::File,
    os::unix::prelude::AsRawFd,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};

extern "C" {
    static FS_IOC_GETFLAGS_EXPORT: libc::c_ulong;
    static FS_IOC_SETFLAGS_EXPORT: libc::c_ulong;
    static FS_IMMUTABLE_FL_EXPORT: libc::c_int;
}

/// Sets the immutability back on drop.
pub struct Guard {
    attr: libc::c_int,
    file: File,
    path: PathBuf,
}

impl Drop for Guard {
    fn drop(&mut self) {
        let fd = self.file.as_raw_fd();

        if unsafe { libc::ioctl(fd, FS_IOC_SETFLAGS_EXPORT, &self.attr) } == -1 {
            let error = std::io::Error::last_os_error();
            log::warn!(
                "Unable make file {} immutable: {:#}",
                self.path.display(),
                error
            );
        } else {
            log::debug!("Immutability of {} has been restored", self.path.display())
        }
    }
}

/// Removes the "immutable" attribute from a file at the given path and returns
/// a [Guard] that will restore the attribute back when dropped.
///
/// If the file didn't have the flag at the first place, [None] is returned,
/// hence the file wouldn't become immutable afterwards.
///
/// For explanation on immutability see `man 1 chattr`.
pub fn make_mutable<P>(path: P) -> Result<Option<Guard>>
where
    P: AsRef<Path>,
{
    let path = path.as_ref();

    let file =
        File::open(path).with_context(|| format!(r#"Unable to open path "{}""#, path.display()))?;
    let fd = file.as_raw_fd();

    let mut original_attr: libc::c_int = 0;
    if unsafe { libc::ioctl(fd, FS_IOC_GETFLAGS_EXPORT, &mut original_attr) } == -1 {
        return Err(std::io::Error::last_os_error())
            .with_context(|| format!("Unable to obtain inode flags on {}", path.display()));
    }
    // Make the variable immutable.
    let original_attr = original_attr;

    if original_attr & unsafe { FS_IMMUTABLE_FL_EXPORT } == 0 {
        // No immutable flag set, move along.
        return Ok(None);
    }

    // Switch off the immutability.
    let new_attr = original_attr ^ unsafe { FS_IMMUTABLE_FL_EXPORT };
    if unsafe { libc::ioctl(fd, FS_IOC_SETFLAGS_EXPORT, &new_attr) } == -1 {
        return Err(std::io::Error::last_os_error())
            .with_context(|| format!("Unable to switch off immutability of {}", path.display()));
    }
    log::debug!("Immutable flag removed from file {}", path.display());

    let guard = Guard {
        attr: original_attr,
        file,
        path: path.to_owned(),
    };

    Ok(Some(guard))
}
