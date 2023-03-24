fn single_slice(bytes: &[u8], index: usize) -> [u8; 4] {
    match bytes[index..index + 4].try_into() {
        Ok(v) => v,
        Err(e) => panic!("{}", e)
    }
}

fn small_slice(bytes: &[u8], index: usize) -> [u8; 2] {
    bytes[index..index + 2].try_into().unwrap()
}

pub fn read_bool(bytes: &[u8], index: usize) -> bool {
    let slice = single_slice(bytes, index);
    u32::from_ne_bytes(slice) > 0
}

pub fn read_u8(bytes: &[u8], index: usize) -> u8 {
    bytes[index]
}

pub fn read_i8(bytes: &[u8], index: usize) -> i8 {
    i8::from_ne_bytes([ bytes[index] ])
}

pub fn read_u16(bytes: &[u8], index: usize) -> u16 {
    let slice = small_slice(bytes, index);
    u16::from_ne_bytes(slice)
}

pub fn read_u32(bytes: &[u8], index: usize) -> u32 {
    let slice = single_slice(bytes, index);
    u32::from_ne_bytes(slice)
}

pub fn read_f32(bytes: &[u8], index: usize) -> f32 {
    let slice = single_slice(bytes, index);
    f32::from_ne_bytes(slice)
}


#[cfg(test)]
mod tests {
    use crate::base::bytes;

    #[test]
    fn can_get_boolean() {
        // Should be true
        let test_bytes = vec![1, 0, 0, 0];
        assert!(bytes::read_bool(&test_bytes, 0));

        // Should be false
        let test_bytes = vec![0, 0, 0, 0];
        assert!(!bytes::read_bool(&test_bytes, 0));
    }

    #[test]
    fn can_get_u16() {
        let test_bytes = vec![0, 1];
        let res = bytes::read_u16(&test_bytes, 0);

        assert!(res == 256);
        assert!(res.to_ne_bytes() == test_bytes.as_slice());
    }

    #[test]
    fn can_get_u32() {
        let test_bytes = vec![1, 0, 0, 0];
        let res = bytes::read_u32(&test_bytes, 0);

        assert!(res == 1);
        assert!(res.to_ne_bytes() == test_bytes.as_slice());
    }
}