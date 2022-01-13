use anyhow::{Context, Result};
use efivar::{
    efi::{VariableFlags, VariableName},
    VarWriter,
};

/// Converts the provided string into a UTF-16 representation and writes it to
/// the given EFI variable, adding terminating null bytes if the original string
/// is not null-terminated.
pub fn write_utf16_string<T: VarWriter + ?Sized>(
    var_manager: &mut T,
    name: &VariableName,
    flags: VariableFlags,
    value: &str,
) -> Result<()> {
    let mut buffer = Vec::with_capacity(value.as_bytes().len() * 2);
    for wide_char in value.encode_utf16() {
        let [first, second] = wide_char.to_le_bytes();
        if first == 0 && second == 0 {
            log::warn!("Intermediate null byte in {:?}", value);
        }
        buffer.push(first);
        buffer.push(second);
    }
    if let [.., 0, 0] = &buffer[..] {
        // Already null-terminated. No need to add trailing zeroes.
    } else {
        // Not null-terminated. Add trailing zeroes!
        buffer.extend_from_slice(&[0, 0]);
    }
    log::trace!("{} encoded as utf16 {:x?}", value, buffer);

    var_manager
        .write(name, flags, &buffer)
        .map_err(crate::error::EfiError)
        .with_context(|| format!("Unable to set variable '{}' to '{}'", name, value))
}
