//! Inode flags (attributes) manipulations.

use std::{
    fs::File,
    os::unix::prelude::AsRawFd,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};

/// A [File] extension trait to allow immutability manipulations.
pub trait FileAttributes {
    /// Returns the currently set inode flags.
    fn inode_flags(&self) -> nix::Result<libc::c_long>;

    /// Updates the inode flags.
    fn set_inode_flags(&self, flags: libc::c_long) -> nix::Result<()>;

    /// Sets or unsets the "immutable" flag.
    fn set_immutable(&self, immutable: bool) -> nix::Result<()>;
}

impl FileAttributes for File {
    fn inode_flags(&self) -> nix::Result<libc::c_long> {
        let mut flags = 0;
        // Safety: the ioctl request is set up correctly.
        unsafe { get_inode_flags(self.as_raw_fd(), &mut flags) }?;
        Ok(flags)
    }

    fn set_inode_flags(&self, flags: libc::c_long) -> nix::Result<()> {
        // Safety: the ioctl request is set up correctly.
        unsafe { set_inode_flags(self.as_raw_fd(), &flags) }?;
        Ok(())
    }

    fn set_immutable(&self, immutable: bool) -> nix::Result<()> {
        let flags = self.inode_flags()?;
        let flags = match (flags & FS_IMMUTABLE_FL == FS_IMMUTABLE_FL, immutable) {
            (true, true) | (false, false) => {
                // Nothing to do.
                return Ok(());
            }
            (true, false) | (false, true) => {
                // Need to switch the flag
                flags ^ FS_IMMUTABLE_FL
            }
        };
        self.set_inode_flags(flags)
    }
}

/// Inode flag "Immutable file".
///
/// See `linux/fs.h`.
const FS_IMMUTABLE_FL: i64 = 0x10;

/// Sets the immutability back on drop.
pub struct Guard {
    attr: i64,
    path: PathBuf,
}

nix::ioctl_read!(get_inode_flags, b'f', 1, libc::c_long);
nix::ioctl_write_ptr!(set_inode_flags, b'f', 2, libc::c_long);

impl Drop for Guard {
    fn drop(&mut self) {
        let file = match std::fs::File::open(&self.path) {
            Ok(file) => file,
            Err(e) => {
                log::warn!(
                    "Unable to open file {} to make it immutable: {:#}",
                    self.path.display(),
                    e
                );
                return;
            }
        };

        if let Err(error) = file.set_inode_flags(self.attr) {
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
pub fn temp_mutable<P>(path: P) -> Result<Option<Guard>>
where
    P: AsRef<Path>,
{
    let path = path.as_ref();

    let file = match File::open(path) {
        Ok(file) => Ok(file),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // It's okay that the file doesn't exist. It will be created when
            // the respective EFI variable is set.
            let guard = Guard {
                attr: FS_IMMUTABLE_FL,
                path: path.to_owned(),
            };
            return Ok(Some(guard));
        }
        Err(e) => Err(e),
    }
    .context("Unable to open the file")?;

    let original_attr = file.inode_flags().context("Unable to obtain inode flags")?;

    if original_attr & FS_IMMUTABLE_FL == 0 {
        // No immutable flag set, move along.
        return Ok(None);
    }

    // Switch off the immutability.
    let new_attr = original_attr ^ FS_IMMUTABLE_FL;
    file.set_inode_flags(new_attr)
        .context("Unable to switch off immutability")?;

    drop(file);

    log::debug!("Immutable flag removed from file {}", path.display());

    let guard = Guard {
        attr: original_attr,
        path: path.to_owned(),
    };

    Ok(Some(guard))
}
