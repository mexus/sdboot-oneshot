use anyhow::{Context, Result};
use efivar::{
    efi::{VariableFlags, VariableName, VariableVendor},
    VarManager,
};
use uuid::Uuid;

mod array_ext;
mod error;
mod read;
mod write;

#[cfg(target_os = "linux")]
mod attributes;

/// Vendor bytes from https://systemd.io/BOOT_LOADER_INTERFACE/
const SYSTEMD_BOOT_VENDOR_RAW: Uuid = Uuid::from_bytes([
    0x4a, 0x67, 0xb0, 0x82, 0x0a, 0x4c, 0x41, 0xcf, 0xb6, 0xc7, 0x44, 0x0b, 0x29, 0xbb, 0x8c, 0x4f,
]);

/// SystemD vendor UUID.
const SYSTEMD_BOOT_VENDOR: VariableVendor = VariableVendor::Custom(SYSTEMD_BOOT_VENDOR_RAW);

/// The EFI variable LoaderEntryOneShot contains the default boot loader entry
/// to use for a single following boot. It is set by the OS in order to request
/// booting into a specific menu entry on the following boot. When set overrides
/// LoaderEntryDefault. It is removed automatically after being read by the boot
/// loader, to ensure it only takes effect a single time. This value is
/// formatted the same way as LoaderEntryDefault.
///
/// (c) https://systemd.io/BOOT_LOADER_INTERFACE/
const ONESHOT_ENTRY_SHORT: &str = "LoaderEntryOneShot";

/// The EFI variable LoaderEntries may contain a series of boot loader entry
/// identifiers, one after the other, each individually NUL terminated. This may
/// be used to let the OS know which boot menu entries were discovered by the
/// boot loader. A boot loader entry identifier should be a short, non-empty
/// alphanumeric string (possibly containing -, too). The list should be in the
/// order the entries are shown on screen during boot. See below regarding a
/// recommended vocabulary for boot loader entry identifiers.
///
/// (c) https://systemd.io/BOOT_LOADER_INTERFACE/
const LOADER_ENTRIES_SHORT: &str = "LoaderEntries";

/// The EFI variable LoaderEntrySelected contains the boot loader entry
/// identifier that was booted. It is set by the boot loader and read by the OS
/// in order to identify which entry has been used for the current boot.
///
/// (c) https://systemd.io/BOOT_LOADER_INTERFACE/
const LOADER_ENTRY_SELECTED: &str = "LoaderEntrySelected";

/// The EFI variable LoaderEntryDefault contains the default boot loader entry
/// to use. It contains a NUL-terminated boot loader entry identifier.
///
/// (c) https://systemd.io/BOOT_LOADER_INTERFACE/
const LOADER_ENTRY_DEFAULT: &str = "LoaderEntryDefault";

/// Systemd-boot entries manager.
pub struct Manager {
    inner: Box<dyn VarManager>,
    oneshot_var: VariableName,
}

// Flags on the oneshot entry EFI variable.
fn oneshot_entry_flags() -> VariableFlags {
    VariableFlags::NON_VOLATILE | VariableFlags::BOOTSERVICE_ACCESS | VariableFlags::RUNTIME_ACCESS
}

impl Manager {
    /// Initializes the manager.
    pub fn new() -> Self {
        Self {
            inner: efivar::system(),
            oneshot_var: VariableName::new_with_vendor(ONESHOT_ENTRY_SHORT, SYSTEMD_BOOT_VENDOR),
        }
    }

    fn get_string(&self, var_name: &str) -> Result<String> {
        let (value, _flags) = read::read_utf16_string(
            &*self.inner,
            &VariableName::new_with_vendor(var_name, SYSTEMD_BOOT_VENDOR),
        )?
        .with_context(|| format!("Variable {} is not set", var_name))?;
        Ok(value)
    }

    /// Returns the entry that was currently booted.
    pub fn get_entry_selected(&self) -> Result<String> {
        self.get_string(LOADER_ENTRY_SELECTED)
    }

    /// Returns the default entry.
    pub fn get_default_entry(&self) -> Result<String> {
        self.get_string(LOADER_ENTRY_DEFAULT)
    }

    /// Fetches the current oneshot entry value.
    pub fn get_oneshot(&self) -> Result<Option<String>> {
        let (value, flags) = match read::read_utf16_string(&*self.inner, &self.oneshot_var)? {
            Some(data) => data,
            None => return Ok(None),
        };

        let expected_flags = oneshot_entry_flags();
        anyhow::ensure!(
            flags == expected_flags,
            "Flags on the oneshot entry ({:?}) differs from expected ({:?})!",
            flags,
            expected_flags
        );
        Ok(Some(value))
    }

    /// Sets value of the oneshot entry.
    pub fn set_oneshot(&mut self, value: &str) -> Result<()> {
        let flags = oneshot_entry_flags();

        // On linux, we want to preserve the "immutable" extended attribute on
        // the variable file, but we need to make it mutable to save the new
        // value temporary.
        #[cfg(target_os = "linux")]
        let _guard = attributes::make_mutable(concat!(
            // Path to the EFI variables storage on linux.
            "/sys/firmware/efi/efivars/",
            // Name of the EFI variable in question.
            "LoaderEntryOneShot",
            // Delimiter.
            "-",
            // SystemD vendor UUID.
            "4a67b082-0a4c-41cf-b6c7-440b29bb8c4f"
        ))?;

        write::write_utf16_string(&mut *self.inner, &self.oneshot_var, flags, value)
    }

    /// Fetches the available entries.
    pub fn entries(&self) -> Result<Vec<String>> {
        let (entries_bytes, _flags) = read::read_u16_bytes(
            &*self.inner,
            &VariableName::new_with_vendor(LOADER_ENTRIES_SHORT, SYSTEMD_BOOT_VENDOR),
        )?
        .with_context(|| format!(r#"Variable {} is not set"#, LOADER_ENTRIES_SHORT))?;
        Ok(entries_bytes
            .split(|&byte| byte == 0)
            .take_while(|entry| {
                // Entries must be non-empty.
                !entry.is_empty()
            })
            .filter_map(|entry| match String::from_utf16(entry) {
                Ok(value) => Some(value),
                Err(_) => {
                    log::warn!(
                        "Discovered an invalid utf16 entry: '{}'; skipping it.",
                        String::from_utf16_lossy(entry)
                    );
                    None
                }
            })
            .collect())
    }
}

impl Default for Manager {
    fn default() -> Self {
        Self::new()
    }
}
