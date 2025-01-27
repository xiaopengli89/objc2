#![cfg(feature = "Foundation_NSMutableDictionary")]
#![cfg(feature = "Foundation_NSNumber")]
use objc2::msg_send;
use objc2::rc::{Id, __RcTestObject, __ThreadTestData};

use icrate::Foundation::{NSMutableDictionary, NSNumber, NSObject};

fn sample_dict() -> Id<NSMutableDictionary<NSNumber, NSObject>> {
    NSMutableDictionary::from_id_slice(
        &[
            &*NSNumber::new_i32(1),
            &*NSNumber::new_i32(2),
            &*NSNumber::new_i32(3),
        ],
        &[NSObject::new(), NSObject::new(), NSObject::new()],
    )
}

#[cfg(feature = "Foundation_NSMutableString")]
fn sample_dict_mut() -> Id<NSMutableDictionary<NSNumber, icrate::Foundation::NSMutableString>> {
    NSMutableDictionary::from_vec(
        &[
            &*NSNumber::new_i32(1),
            &*NSNumber::new_i32(2),
            &*NSNumber::new_i32(3),
        ],
        vec![
            icrate::Foundation::NSMutableString::from_str("a"),
            icrate::Foundation::NSMutableString::from_str("b"),
            icrate::Foundation::NSMutableString::from_str("c"),
        ],
    )
}

#[test]
#[cfg(feature = "Foundation_NSMutableString")]
fn dict_from_mutable() {
    let _: Id<NSMutableDictionary<icrate::Foundation::NSString, icrate::Foundation::NSString>> =
        NSMutableDictionary::from_id_slice(
            &[&*icrate::Foundation::NSMutableString::from_str("a")],
            &[Id::into_super(
                icrate::Foundation::NSMutableString::from_str("b"),
            )],
        );
}

#[test]
fn test_new() {
    let dict = NSMutableDictionary::<NSObject, NSObject>::new();
    assert!(dict.is_empty());
}

#[test]
#[cfg(feature = "Foundation_NSMutableString")]
fn test_get_mut() {
    let mut dict = sample_dict_mut();

    assert!(dict.get_mut(&NSNumber::new_i32(1)).is_some());
    assert!(dict.get_mut(&NSNumber::new_i32(2)).is_some());
    assert!(dict.get_mut(&NSNumber::new_i32(4)).is_none());
}

#[test]
#[cfg(feature = "Foundation_NSMutableString")]
fn test_values_mut() {
    let mut dict = sample_dict_mut();
    let vec = dict.values_vec_mut();
    assert_eq!(vec.len(), 3);
}

#[test]
fn test_insert() {
    let mut dict = <NSMutableDictionary<NSNumber, _>>::new();
    assert!(dict
        .insert_id(&NSNumber::new_i32(1), NSObject::new())
        .is_none());
    assert!(dict
        .insert_id(&NSNumber::new_i32(2), NSObject::new())
        .is_none());
    assert!(dict
        .insert_id(&NSNumber::new_i32(3), NSObject::new())
        .is_none());
    assert!(dict
        .insert_id(&NSNumber::new_i32(1), NSObject::new())
        .is_some());
    assert_eq!(dict.len(), 3);
}

#[test]
fn test_insert_key_copies() {
    let mut dict = <NSMutableDictionary<__RcTestObject, _>>::new();
    let key1 = __RcTestObject::new();
    let mut expected = __ThreadTestData::current();

    let _ = dict.insert_id(&key1, NSNumber::new_i32(1));
    // Create copy
    expected.copy += 1;
    expected.alloc += 1;
    expected.init += 1;
    expected.assert_current();

    dict.removeAllObjects();
    // Release key
    expected.release += 1;
    expected.dealloc += 1;
    expected.assert_current();
}

#[test]
fn test_get_key_copies() {
    let mut dict = <NSMutableDictionary<__RcTestObject, _>>::new();
    let key1 = __RcTestObject::new();
    let _ = dict.insert_id(&key1, NSNumber::new_i32(1));
    let expected = __ThreadTestData::current();

    let _ = dict.get(&key1);
    // No change, getting doesn't do anything to the key!
    expected.assert_current();
}

#[test]
fn test_insert_value_retain_release() {
    let mut dict = <NSMutableDictionary<NSNumber, _>>::new();
    dict.insert_id(&NSNumber::new_i32(1), __RcTestObject::new());
    let to_insert = __RcTestObject::new();
    let mut expected = __ThreadTestData::current();

    let old = dict.insert(&NSNumber::new_i32(1), &to_insert);
    // Grab old value
    expected.retain += 1;

    // Dictionary takes new value and overwrites the old one
    expected.retain += 1;
    expected.release += 1;

    expected.assert_current();

    drop(old);
    expected.release += 1;
    expected.dealloc += 1;
    expected.assert_current();
}

#[test]
fn test_remove() {
    let mut dict = sample_dict();
    assert_eq!(dict.len(), 3);
    assert!(dict.remove(&NSNumber::new_i32(1)).is_some());
    assert!(dict.remove(&NSNumber::new_i32(2)).is_some());
    assert!(dict.remove(&NSNumber::new_i32(1)).is_none());
    assert!(dict.remove(&NSNumber::new_i32(4)).is_none());
    assert_eq!(dict.len(), 1);
}

#[test]
fn test_clear() {
    let mut dict = sample_dict();
    assert_eq!(dict.len(), 3);

    dict.removeAllObjects();
    assert!(dict.is_empty());
}

#[test]
fn test_remove_clear_release_dealloc() {
    let mut dict = <NSMutableDictionary<NSNumber, _>>::new();
    for i in 0..4 {
        dict.insert_id(&NSNumber::new_i32(i), __RcTestObject::new());
    }
    let mut expected = __ThreadTestData::current();

    let _obj = dict.remove(&NSNumber::new_i32(1));
    expected.retain += 1;
    expected.release += 1;
    expected.assert_current();
    assert_eq!(dict.len(), 3);

    let _obj = dict.remove(&NSNumber::new_i32(2));
    expected.retain += 1;
    expected.release += 1;
    expected.assert_current();
    assert_eq!(dict.len(), 2);

    dict.removeAllObjects();
    expected.release += 2;
    expected.dealloc += 2;
    expected.assert_current();
    assert_eq!(dict.len(), 0);
}

#[test]
#[cfg(feature = "Foundation_NSArray")]
fn test_to_array() {
    let dict = sample_dict();
    let array = dict.to_array();
    assert_eq!(array.len(), 3);
}

#[test]
#[should_panic = "mutation detected during enumeration"]
#[cfg_attr(
    not(debug_assertions),
    ignore = "enumeration mutation only detected with debug assertions on"
)]
fn test_iter_mutation_detection() {
    let dict = sample_dict();

    let mut iter = dict.keys();
    let _ = iter.next();

    let key: &NSNumber = &NSNumber::new_usize(1);
    let obj: &NSObject = &NSObject::new();
    let _: () = unsafe { msg_send![&dict, setObject: obj, forKey: key] };

    let _ = iter.next();
}
