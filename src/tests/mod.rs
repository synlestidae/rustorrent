use bencode::decode::{bint_decode};

#[test]
pub fn test_decodes_int_0() {
    assert_eq!(decode_str_to_u64("i0e"), 0);
}

#[test]
pub fn test_negative_zero_not_decoded() {
    assert!(!bint_decode("i-0e".to_string().into_bytes()).is_ok());
}

#[test]
pub fn test_decodes_int_1() {
    assert_eq!(decode_str_to_u64("i1e"), 1);
}

#[test]
pub fn test_decodes_int_3() {
    assert_eq!(decode_str_to_u64("i3e"), 3);
}

#[test]
pub fn test_decodes_int_11() {
    assert_eq!(decode_str_to_u64("i11e"), 11);
}

/*#[test]
pub fn test_decodes_int_neg_1() {
    assert_eq!(decode_str_to_u64("i-1e"), -1);
}

#[test]
pub fn test_decodes_int_neg_2() {
    assert_eq!(decode_str_to_u64("i-2e"), -2);
}

#[test]
pub fn test_decodes_int_neg_3() {
    assert_eq!(decode_str_to_u64("i-1e"), -3);
}

#[test]
pub fn test_decodes_int_neg_11() {
    assert_eq!(decode_str_to_u64("i-11e"), -11);
}*/




fn decode_str_to_u64(s: &str) -> u64 {
   let result = bint_decode(s.to_string().into_bytes());
   result.ok().unwrap().to_u64()
}
