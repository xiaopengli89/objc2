//! Test how static selectors work in relation to `msg_send!` and `msg_send_id!`
#![cfg(feature = "apple")]
use objc2::rc::Id;
use objc2::runtime::{AnyClass, AnyObject, Sel};
use objc2::{msg_send, msg_send_id, sel};

#[no_mangle]
unsafe fn handle_with_sel(obj: &AnyObject) -> *mut AnyObject {
    msg_send![obj, someSelector]
}

#[no_mangle]
unsafe fn handle_alloc_init(cls: &AnyClass) -> Id<AnyObject> {
    msg_send_id![msg_send_id![cls, alloc], init]
}

fn generic<T>() -> Sel {
    sel!(generic:selector:)
}

#[no_mangle]
unsafe fn use_generic(obj: &AnyObject) {
    let _: () = msg_send![obj, performSelector: generic::<i32>()];
    let _: () = msg_send![obj, performSelector: generic::<u32>()];
    let _: () = msg_send![obj, performSelector: generic::<f32>()];
}
