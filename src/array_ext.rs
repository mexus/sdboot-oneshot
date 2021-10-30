/// A helper trait to access [u16] slices/arrays as double-sized [u8]
/// slices/arrays respectively.
pub trait U16ArrayExt {
    /// Returns a reference to the object as a [u8] slice.
    fn as_u8(&self) -> &[u8];

    /// Returns a mutable reference to the object as a mutable [u8] slice.
    fn as_u8_mut(&mut self) -> &mut [u8];
}

impl U16ArrayExt for [u16] {
    fn as_u8(&self) -> &[u8] {
        let length = self.len().checked_mul(2).expect("Integer overflow");
        let begin = self.as_ptr().cast::<u8>();
        // The operation is safe because u16 has stronger align requirements
        // than u8, and size of u16 is two bytes (hence length is multiplied).
        unsafe { std::slice::from_raw_parts(begin, length) }
    }

    fn as_u8_mut(&mut self) -> &mut [u8] {
        let length = self.len().checked_mul(2).expect("Integer overflow");
        let begin = self.as_mut_ptr().cast::<u8>();
        // The operation is safe because u16 has stronger align requirements
        // than u8, and size of u16 is two bytes (hence length is multiplied).
        unsafe { std::slice::from_raw_parts_mut(begin, length) }
    }
}
