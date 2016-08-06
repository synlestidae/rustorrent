#[allow(unused_imports)]
use bencode::BString;
use bencode::decode::{bint_decode, bstring_decode};

#[test]
pub fn test_decodes_int_0() {
    assert_eq!(decode_str_to_i64("i0e"), 0);
}

#[test]
pub fn test_negative_zero_not_decoded() {
    assert!(!bint_decode(&"i-0e".to_string().into_bytes(), &mut 0).is_ok());
}

#[test]
pub fn test_decodes_int_1() {
    assert_eq!(decode_str_to_i64("i1e"), 1);
}

#[test]
pub fn test_decodes_int_3() {
    assert_eq!(decode_str_to_i64("i3e"), 3);
}

#[test]
pub fn test_decodes_int_11() {
    assert_eq!(decode_str_to_i64("i11e"), 11);
}

#[test]
pub fn test_decodes_int_neg_1() {
    assert_eq!(decode_str_to_i64("i-1e"), -1);
}

#[test]
pub fn test_decodes_int_neg_2() {
    assert_eq!(decode_str_to_i64("i-2e"), -2);
}

#[test]
pub fn test_decodes_int_neg_3() {
    assert_eq!(decode_str_to_i64("i-3e"), -3);
}

#[test]
pub fn test_decodes_int_neg_11() {
    assert_eq!(decode_str_to_i64("i-11e"), -11);
}

#[test]
pub fn test_decodes_32bit_integer_range() {
    let step = 21481;
    let mut i = i32::min_value();
    while (i <= i32::max_value() - step) {
        assert_eq!(decode_str_to_i64(&format!("i{}e", i)), i as i64);
        i += step;
    }
}

#[test]
pub fn test_decodes_hello_world_string() {
    test_decode_str("Hello, world!", "13:Hello, world!");
}

#[cfg(test)]
fn decode_str_to_i64(s: &str) -> i64 {
    let result = bint_decode(&s.to_string().into_bytes(), &mut 0);
    result.ok().unwrap().to_i64()
}

#[cfg(test)]
fn test_decode_str(expected: &str, actual: &str) {
    let bstring = bstring_decode(&actual.to_string().into_bytes(), &mut 0).ok().unwrap();
    assert_eq!(expected.to_string(), bstring.to_string().ok().unwrap());
}
