use alloc::string::{String, ToString};
use core::fmt;
use core::mem;
use core::ptr::NonNull;
use std::error::Error;

use crate::rc::{Id, Ownership};
use crate::runtime::{Class, Imp, Object, Sel};
use crate::{Encode, EncodeArguments, RefEncode};

#[cfg(feature = "exception")]
unsafe fn conditional_try<R: Encode>(f: impl FnOnce() -> R) -> Result<R, MessageError> {
    use alloc::borrow::ToOwned;
    crate::exception::catch_exception(f).map_err(|exception| {
        if let Some(exception) = exception {
            MessageError(alloc::format!("Uncaught exception {:?}", exception))
        } else {
            MessageError("Uncaught exception nil".to_owned())
        }
    })
}

#[cfg(not(feature = "exception"))]
#[inline(always)]
unsafe fn conditional_try<R: Encode>(f: impl FnOnce() -> R) -> Result<R, MessageError> {
    Ok(f())
}

mod verify;

#[cfg(target_vendor = "apple")]
#[path = "apple/mod.rs"]
mod platform;
#[cfg(not(target_vendor = "apple"))]
#[path = "gnustep.rs"]
mod platform;

use self::platform::{send_super_unverified, send_unverified};
use self::verify::{verify_message_signature, VerificationError};

/// Types that can be sent Objective-C messages.
///
/// Examples include objects, classes, and blocks.
///
/// Implementing this provides [`MessageReceiver`] implementations for common
/// pointer types and references to the type, which allows using them as the
/// receiver (first argument) in the [`msg_send!`][`crate::msg_send`] macro.
///
/// # Safety
///
/// A pointer to the type must be able to be the receiver of an Objective-C
/// message sent with [`objc_msgSend`] or similar.
///
/// Additionally, the type must implement [`RefEncode`] and adhere to the
/// safety requirements therein.
///
/// [`objc_msgSend`]: https://developer.apple.com/documentation/objectivec/1456712-objc_msgsend
pub unsafe trait Message: RefEncode {}

unsafe impl Message for Object {}

unsafe impl Message for Class {}

// TODO: Make this fully private
pub(crate) mod private {
    use super::{Id, Message, NonNull, Ownership};

    pub trait Sealed {}

    impl<T: Message + ?Sized> Sealed for *const T {}
    impl<T: Message + ?Sized> Sealed for *mut T {}

    impl<'a, T: Message + ?Sized> Sealed for &'a T {}
    impl<'a, T: Message + ?Sized> Sealed for &'a mut T {}
    impl<T: Message + ?Sized> Sealed for NonNull<T> {}
    impl<T: Message, O: Ownership> Sealed for Id<T, O> {}
}

/// Types that can directly be used as the receiver of Objective-C messages.
///
/// This is a sealed trait (for now) that is automatically implemented for
/// pointers to types implementing [`Message`], so that code can be generic
/// over the message receiver.
///
/// This is mostly an implementation detail; you'll want to implement
/// [`Message`] for your type instead.
///
/// # Safety
///
/// [`Self::as_raw_receiver`] must be implemented correctly.
pub unsafe trait MessageReceiver: private::Sealed {
    /// Get a raw pointer to the receiver of the message.
    fn as_raw_receiver(&self) -> *mut Object;

    /// Sends a message to self with the given selector and arguments.
    ///
    /// The correct version of `objc_msgSend` will be chosen based on the
    /// return type. For more information, see the section on "Sending
    /// Messages" in Apple's [documentation][runtime].
    ///
    /// If the selector is known at compile-time, it is recommended to use the
    /// [`msg_send!`][`crate::msg_send`] macro rather than this method.
    ///
    /// [runtime]: https://developer.apple.com/documentation/objectivec/objective-c_runtime?language=objc
    #[cfg_attr(feature = "verify_message", inline(always))]
    unsafe fn send_message<A, R>(&self, sel: Sel, args: A) -> Result<R, MessageError>
    where
        A: MessageArguments,
        R: Encode,
    {
        let this = self.as_raw_receiver();
        #[cfg(feature = "verify_message")]
        {
            let cls = if this.is_null() {
                return Err(VerificationError::NilReceiver(sel).into());
            } else {
                (*(this as *const Object)).class()
            };

            verify_message_signature::<A, R>(cls, sel)?;
        }
        send_unverified(this, sel, args)
    }

    /// Sends a message to self's superclass with the given selector and
    /// arguments.
    ///
    /// The correct version of `objc_msgSend_super` will be chosen based on the
    /// return type. For more information, see the section on "Sending
    /// Messages" in Apple's [documentation][runtime].
    ///
    /// If the selector is known at compile-time, it is recommended to use the
    /// [`msg_send!(super)`][`crate::msg_send`] macro rather than this method.
    ///
    /// [runtime]: https://developer.apple.com/documentation/objectivec/objective-c_runtime?language=objc
    #[cfg_attr(feature = "verify_message", inline(always))]
    unsafe fn send_super_message<A, R>(
        &self,
        superclass: &Class,
        sel: Sel,
        args: A,
    ) -> Result<R, MessageError>
    where
        A: MessageArguments,
        R: Encode,
    {
        let this = self.as_raw_receiver();
        #[cfg(feature = "verify_message")]
        {
            if this.is_null() {
                return Err(VerificationError::NilReceiver(sel).into());
            }
            verify_message_signature::<A, R>(superclass, sel)?;
        }
        send_super_unverified(this, superclass, sel, args)
    }

    /// Verify that the argument and return types match the encoding of the
    /// method for the given selector.
    ///
    /// This will look up the encoding of the method for the given selector,
    /// `sel`, and return a [`MessageError`] if any encodings differ for the
    /// arguments `A` and return type `R`.
    ///
    /// # Example
    /// ``` no_run
    /// # use objc2::{class, msg_send, sel};
    /// # use objc2::runtime::{BOOL, Class, Object};
    /// # use objc2::MessageReceiver;
    /// let obj: &Object;
    /// # obj = unsafe { msg_send![class!(NSObject), new] };
    /// let sel = sel!(isKindOfClass:);
    /// // Verify isKindOfClass: takes one Class and returns a BOOL
    /// let result = obj.verify_message::<(&Class,), BOOL>(sel);
    /// assert!(result.is_ok());
    /// ```
    fn verify_message<A, R>(&self, sel: Sel) -> Result<(), MessageError>
    where
        A: EncodeArguments,
        R: Encode,
    {
        let obj = unsafe { &*self.as_raw_receiver() };
        verify_message_signature::<A, R>(obj.class(), sel).map_err(MessageError::from)
    }
}

// Note that we implement MessageReceiver for unsized types as well, this is
// to support `extern type`s in the future, not because we want to allow DSTs.

unsafe impl<T: Message + ?Sized> MessageReceiver for *const T {
    #[inline]
    fn as_raw_receiver(&self) -> *mut Object {
        *self as *mut T as *mut Object
    }
}

unsafe impl<T: Message + ?Sized> MessageReceiver for *mut T {
    #[inline]
    fn as_raw_receiver(&self) -> *mut Object {
        *self as *mut Object
    }
}

unsafe impl<'a, T: Message + ?Sized> MessageReceiver for &'a T {
    #[inline]
    fn as_raw_receiver(&self) -> *mut Object {
        *self as *const T as *mut T as *mut Object
    }
}

unsafe impl<'a, T: Message + ?Sized> MessageReceiver for &'a mut T {
    #[inline]
    fn as_raw_receiver(&self) -> *mut Object {
        *self as *const T as *mut T as *mut Object
    }
}

unsafe impl<T: Message + ?Sized> MessageReceiver for NonNull<T> {
    #[inline]
    fn as_raw_receiver(&self) -> *mut Object {
        self.as_ptr() as *mut Object
    }
}

unsafe impl<T: Message, O: Ownership> MessageReceiver for Id<T, O> {
    #[inline]
    fn as_raw_receiver(&self) -> *mut Object {
        // TODO: Maybe don't dereference here, just to be safe?
        (&**self).as_raw_receiver()
    }
}

/// Types that may be used as the arguments of an Objective-C message.
pub trait MessageArguments: EncodeArguments {
    /// Invoke an [`Imp`] with the given object, selector, and arguments.
    ///
    /// This method is the primitive used when sending messages and should not
    /// be called directly; instead, use the `msg_send!` macro or, in cases
    /// with a dynamic selector, the [`MessageReceiver::send_message`] method.
    unsafe fn invoke<R: Encode>(imp: Imp, obj: *mut Object, sel: Sel, args: Self) -> R;
}

macro_rules! message_args_impl {
    ($($a:ident : $t:ident),*) => (
        impl<$($t: Encode),*> MessageArguments for ($($t,)*) {
            #[inline]
            unsafe fn invoke<R: Encode>(imp: Imp, obj: *mut Object, sel: Sel, ($($a,)*): Self) -> R {
                // The imp must be cast to the appropriate function pointer
                // type before being called; the msgSend functions are not
                // parametric, but instead "trampolines" to the actual
                // method implementations.
                let imp: unsafe extern "C" fn(*mut Object, Sel $(, $t)*) -> R = mem::transmute(imp);
                imp(obj, sel $(, $a)*)
            }
        }
    );
}

message_args_impl!();
message_args_impl!(a: A);
message_args_impl!(a: A, b: B);
message_args_impl!(a: A, b: B, c: C);
message_args_impl!(a: A, b: B, c: C, d: D);
message_args_impl!(a: A, b: B, c: C, d: D, e: E);
message_args_impl!(a: A, b: B, c: C, d: D, e: E, f: F);
message_args_impl!(a: A, b: B, c: C, d: D, e: E, f: F, g: G);
message_args_impl!(a: A, b: B, c: C, d: D, e: E, f: F, g: G, h: H);
message_args_impl!(a: A, b: B, c: C, d: D, e: E, f: F, g: G, h: H, i: I);
message_args_impl!(a: A, b: B, c: C, d: D, e: E, f: F, g: G, h: H, i: I, j: J);
message_args_impl!(
    a: A,
    b: B,
    c: C,
    d: D,
    e: E,
    f: F,
    g: G,
    h: H,
    i: I,
    j: J,
    k: K
);
message_args_impl!(
    a: A,
    b: B,
    c: C,
    d: D,
    e: E,
    f: F,
    g: G,
    h: H,
    i: I,
    j: J,
    k: K,
    l: L
);

/**
An error encountered while attempting to send a message.

Currently, an error may be returned in two cases:

* an Objective-C exception is thrown and the `exception` feature is enabled
* the encodings of the arguments do not match the encoding of the method
  and the `verify_message` feature is enabled
*/
#[derive(Debug)]
pub struct MessageError(String);

impl fmt::Display for MessageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl Error for MessageError {
    fn description(&self) -> &str {
        &self.0
    }
}

impl<'a> From<VerificationError<'a>> for MessageError {
    fn from(err: VerificationError<'_>) -> MessageError {
        MessageError(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils;

    #[test]
    fn test_send_message() {
        let obj = test_utils::custom_object();
        let result: u32 = unsafe {
            let _: () = msg_send![obj, setFoo: 4u32];
            msg_send![obj, foo]
        };
        assert!(result == 4);
    }

    #[test]
    fn test_send_message_stret() {
        let obj = test_utils::custom_object();
        let result: test_utils::CustomStruct = unsafe { msg_send![obj, customStruct] };
        let expected = test_utils::CustomStruct {
            a: 1,
            b: 2,
            c: 3,
            d: 4,
        };
        assert!(result == expected);
    }

    #[cfg(not(feature = "verify_message"))]
    #[test]
    fn test_send_message_nil() {
        let nil: *mut Object = ::core::ptr::null_mut();
        let result: usize = unsafe { msg_send![nil, hash] };
        assert!(result == 0);

        let result: *mut Object = unsafe { msg_send![nil, description] };
        assert!(result.is_null());

        let result: f64 = unsafe { msg_send![nil, doubleValue] };
        assert!(result == 0.0);
    }

    #[test]
    fn test_send_message_super() {
        let obj = test_utils::custom_subclass_object();
        let superclass = test_utils::custom_class();
        unsafe {
            let _: () = msg_send![obj, setFoo: 4u32];
            let foo: u32 = msg_send![super(obj, superclass), foo];
            assert!(foo == 4);

            // The subclass is overriden to return foo + 2
            let foo: u32 = msg_send![obj, foo];
            assert!(foo == 6);
        }
    }

    #[test]
    fn test_verify_message() {
        let obj = test_utils::custom_object();
        assert!(obj.verify_message::<(), u32>(sel!(foo)).is_ok());
        assert!(obj.verify_message::<(u32,), ()>(sel!(setFoo:)).is_ok());

        // Incorrect types
        assert!(obj.verify_message::<(), u64>(sel!(setFoo:)).is_err());
        // Unimplemented selector
        assert!(obj.verify_message::<(u32,), ()>(sel!(setFoo)).is_err());
    }
}