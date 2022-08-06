pub(crate) fn get_bits_at_from_msb(data: &[u8], position: usize) -> i32 {
    let position = position as i32;
    let t = (data[(position / 8) as usize] as i32) & (1 << (8 - 1 - ((position as u32) % 8)));
    if t > 0 {
        return 1;
    }
    0
}

pub(crate) fn set_bits_at_from_msb(data: &mut [u8], position: usize) {
    let mut n: i32 = data[position / 8] as i32;
    n |= 1 << (8 - 1 - ((position as u32) % 8));
    data[position / 8] = n as u8;
}

pub(crate) fn count_set_bits(data: &[u8]) -> i32 {
    let mut count = 0;
    for i in 0..data.len() * 8 {
        if get_bits_at_from_msb(data, i) == 1 {
            count += 1
        } else {
            break;
        }
    }
    count
}

pub(crate) fn count_common_prefix(lhs: &[u8], rhs: &[u8]) -> i32 {
    let mut count = 0;
    for i in 0..lhs.len() * 8 {
        if get_bits_at_from_msb(lhs, i) == get_bits_at_from_msb(rhs, i) {
            count += 1
        } else {
            break;
        }
    }
    count
}
