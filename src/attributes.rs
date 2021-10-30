//! Inode flags (attributes) manipulations.

use std::{
    fs::File,
    os::unix::prelude::AsRawFd,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};

#[derive(Debug, Clone, Copy)]
enum Direction {
    Write,
    Read,
}

impl Direction {
    const fn as_value(self) -> libc::c_ulong {
        match self {
            Direction::Write => 1,
            Direction::Read => 2,
        }
    }
}

/// Creates an IOCTL request (command) from the given parameters.
///
/// See `ioctl.h` for implementation details.
///
/// # Arguments
///
/// * `direction`: is it "reading" or "writing" command?
/// * `number`: number of the ioctl command.
/// * `type_`: type of the ioctl command.
const fn make_ioctl_request(
    direction: Direction,
    number: libc::c_ulong,
    type_: libc::c_ulong,
) -> libc::c_ulong {
    // From `ioctl.h`:
    //
    // ioctl command encoding: 32 bits total, command in lower 16 bits, size of
    // the parameter structure in the lower 14 bits of the upper 16 bits.
    // Encoding the size of the parameter structure in the ioctl request is
    // useful for catching programs compiled with old versions and to avoid
    // overwriting user space outside the user buffer area. The highest 2 bits
    // are reserved for indicating the ``access mode''.
    //
    // NOTE: This limits the max parameter size to 16kB -1 !

    const NUMBER_SHIFT: libc::c_ulong = 0;
    const NUMBER_BITS: libc::c_ulong = 8;

    const TYPE_SHIFT: libc::c_ulong = NUMBER_BITS + NUMBER_SHIFT;
    const TYPE_BITS: libc::c_ulong = 8;

    const SIZE_SHIFT: libc::c_ulong = TYPE_SHIFT + TYPE_BITS;
    const SIZE_BITS: libc::c_ulong = 14;

    const DIRECTION_SHIFT: libc::c_ulong = SIZE_SHIFT + SIZE_BITS;

    (direction.as_value() << DIRECTION_SHIFT)
        | (type_ << TYPE_SHIFT)
        | (number << NUMBER_SHIFT)
        | ((std::mem::size_of::<libc::c_ulong>() as libc::c_ulong) << SIZE_SHIFT)
}

/// An IOCTL request to get inode flags.
const FS_IOC_GETFLAGS: libc::c_ulong = make_ioctl_request(Direction::Read, 1, b'f' as u64);

/// An IOCTL request to set inode flags.
const FS_IOC_SETFLAGS: libc::c_ulong = make_ioctl_request(Direction::Write, 2, b'f' as u64);

/// Inode flag "Immutable file".
///
/// See `linux/fs.h`.
const FS_IMMUTABLE_FL: libc::c_int = 0x10;

/// Sets the immutability back on drop.
pub struct Guard {
    attr: libc::c_int,
    file: File,
    path: PathBuf,
}

impl Drop for Guard {
    fn drop(&mut self) {
        let fd = self.file.as_raw_fd();

        if unsafe { libc::ioctl(fd, FS_IOC_SETFLAGS, &self.attr) } == -1 {
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
    if unsafe { libc::ioctl(fd, FS_IOC_GETFLAGS, &mut original_attr) } == -1 {
        return Err(std::io::Error::last_os_error())
            .with_context(|| format!("Unable to obtain inode flags on {}", path.display()));
    }
    // Make the variable immutable.
    let original_attr = original_attr;

    if original_attr & FS_IMMUTABLE_FL == 0 {
        // No immutable flag set, move along.
        return Ok(None);
    }

    // Switch off the immutability.
    let new_attr = original_attr ^ FS_IMMUTABLE_FL;
    if unsafe { libc::ioctl(fd, FS_IOC_SETFLAGS, &new_attr) } == -1 {
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
