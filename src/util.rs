pub fn read_variable_length(data: &[u8; 4]) -> (u8, usize) {
    let mut bytes: u8 = 0;
    let mut value: usize = 0;

    for (i, &n) in data.iter().enumerate() {
        value = (value << 7) + (n & 0x7f) as usize;
        if n & 0x80 != 0x80 {
            bytes = (i + 1) as u8;
            break;
        }
    }

    (bytes, value)
}

#[inline(always)]
pub fn tempo2qpm(tempo: u32) -> f32 {
    6e7 / tempo as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_vlq() {
        assert!(read_variable_length(&([0x40u8, 0x00u8, 0x00u8, 0x00u8])).1 == 0x40usize);
        assert!(read_variable_length(&([0xC0u8, 0x00u8, 0x00u8, 0x00u8])).1 == 0x2000usize);
        assert!(read_variable_length(&([0x81u8, 0x80u8, 0x00u8, 0x00u8])).1 == 0x4000usize);
        assert!(read_variable_length(&([0xFFu8, 0xFFu8, 0x7Fu8, 0x00u8])).1 == 0x1FFFFFusize);
    }
}
