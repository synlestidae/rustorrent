mod metainfo;

#[allow(unused_imports)]
use bencode::{BString, Bencode, BInt, BList};
#[allow(unused_imports)]
use bencode::decode::{bint_decode, bstring_decode, bdict_decode, DecodeResult, blist_decode};

#[test]
pub fn test_decodes_int_0() {
    assert_eq!(decode_str_to_i64("i0e"), 0);
}

#[test]
pub fn test_negative_zero_not_decoded() {
    assert!(!bint_decode(&"i-0e".to_string().into_bytes()).is_ok());
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
    while i <= i32::max_value() - step {
        assert_eq!(decode_str_to_i64(&format!("i{}e", i)), i as i64);
        i += step;
    }
}

#[test]
pub fn test_decodes_hello_world_string() {
    test_decode_str("Hello, world!", "13:Hello, world!");
}

#[test]
pub fn test_decodes_dict() {
    let dict = bdict_decode(&"d3:cow3:moo4:spam4:eggse".to_string().into_bytes()).unwrap().0;
    assert_eq!(dict.get("cow").unwrap(),
               &Bencode::BString(BString::from_str("moo")));
}

#[test]
pub fn test_decodes_dict_with_dict_field() {
    let dict =
        bdict_decode(&"d4:listl4:worde3:cow3:moo4:spam4:eggse".to_string().into_bytes()).unwrap().0;
    let list = dict.get("list").unwrap();
    match list {
        &Bencode::BList(ref blist) => {
            assert_eq!(blist.list()[0], Bencode::BString(BString::from_str("word")))
        }
        _ => panic!("Wrong thing: {:?}", list),
    }
}

#[test]
pub fn test_decodes_dict_with_nested_list() {
    let dict = bdict_decode(&"d4:listlleee".to_string().into_bytes()).unwrap().0;
    let list = dict.get("list").unwrap();
    match list {
        &Bencode::BList(ref blist) => {
            match blist.list()[0] {
                Bencode::BList(ref blist) => assert_eq!(0, blist.list().len()),
                _ => panic!("Wrong kind of thing!"),
            }
        }
        _ => panic!("Wrong thing: {:?}", list),
    }
}

#[test]
pub fn test_decodes_list() {
    let list = blist_decode(&"ll1:e2:eeee".to_string().into_bytes()).unwrap();

    let mut expected_list_inner = BList::new();
    let mut expected_list_outer = BList::new();
    expected_list_inner.push(Bencode::BString(BString::from_str("e")));
    expected_list_inner.push(Bencode::BString(BString::from_str("ee")));
    expected_list_outer.push(Bencode::BList(expected_list_inner));

    assert_eq!(expected_list_outer, list.0);
    assert_eq!(11, list.1);
}

#[test]
pub fn test_decodes_dict_with_nested_list_2() {
    let dict =
        bdict_decode(&"d4:listll1:e2:eeee5:thing6:thing1e".to_string().into_bytes()).unwrap().0;
    let list = dict.get("thing").unwrap();
    match list {
        &Bencode::BString(ref bstr) => {
            assert_eq!("thing1".to_string(), bstr.to_string().ok().unwrap());
        }
        _ => panic!("Wrong thing: {:?}", list),
    }
}

#[test]
pub fn test_decodes_list_with_dict() {
    let list_result = blist_decode("ldee".as_bytes()).unwrap();
    assert_eq!(4, list_result.1);
}

#[cfg(test)]
fn decode_str_to_i64(s: &str) -> i64 {
    let result = bint_decode(&s.to_string().into_bytes());
    let result = result.ok().unwrap();
    assert_eq!(s.len(), result.1);
    result.0.to_i64()
}

#[cfg(test)]
fn test_decode_str(expected: &str, actual: &str) {
    let bstring_bytes = actual.to_string().into_bytes();
    let bstring = bstring_decode(&bstring_bytes).ok().unwrap();
    assert_eq!(bstring.1, actual.len());
    assert_eq!(expected.to_string(), bstring.0.to_string().ok().unwrap());
}
