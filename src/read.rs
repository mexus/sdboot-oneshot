use anyhow::{Context, Result};
use efivar::{
    efi::{VariableFlags, VariableName},
    VarReader,
};

use crate::array_ext::U16ArrayExt;

/// Divide `dividend` by `divisor` and round the result up.
const fn divide_up(dividend: usize, divisor: usize) -> usize {
    dividend / divisor + (dividend % divisor != 0) as usize
}

#[cfg(test)]
#[test]
fn check_division() {
    assert_eq!(divide_up(16, 2), 8);
    assert_eq!(divide_up(15, 2), 8);
}

/// Reads the value of the given EFI variable into a vector over [u16].
pub fn read_u16_bytes<T: VarReader + ?Sized>(
    var_manager: &T,
    name: &VariableName,
) -> Result<(Vec<u16>, VariableFlags)> {
    // 8 MBs (when applied to u16).
    const MAX_BUFFER: usize = 8 * 512 * 1024;

    // Start with 1 KB (u16!!!).
    let mut buffer = vec![0u16; 512];
    loop {
        match var_manager.read(name, buffer.as_u8_mut()) {
            Ok((length, flags)) => {
                // The length refers to the u8 buffer, the u16 buffer will be
                // twice as short. If read odd number of bytes, add an extra u16
                // on top of the halved value.
                buffer.resize(divide_up(length, 2), 0);
                break Ok((buffer, flags));
            }
            Err(efivar::Error::BufferTooSmall { .. }) => {
                if buffer.len() >= MAX_BUFFER {
                    // Refuse to grow the buffer beyond MAX_BUFFER.
                    anyhow::bail!(
                        "Unable to read variable {} cause its size of the variable is greater than {} bytes!",
                        name,
                        buffer.len() * 2
                    )
                }
                buffer.resize(buffer.len() * 2, 0);
            }
            Err(e) => {
                break Err(crate::error::EfiError(e))
                    .with_context(|| format!("Reading variable {}", name))
            }
        }
    }
}

/// Reads the value of the given EFI variable as a UTF-16 string.
pub fn read_utf16_string<T: VarReader + ?Sized>(
    var_manager: &T,
    name: &VariableName,
) -> Result<(String, VariableFlags)> {
    let (bytes, flags) = read_u16_bytes(var_manager, name)?;
    let value = String::from_utf16(&bytes)
        .with_context(|| format!("Non-UTF16 value: {}", String::from_utf16_lossy(&bytes)))?;
    Ok((value, flags))
}
