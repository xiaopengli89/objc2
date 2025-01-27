#[cfg(all(debug_assertions, feature = "verify"))]
use alloc::vec::Vec;
use core::marker::PhantomData;
#[cfg(all(debug_assertions, feature = "verify"))]
use std::collections::HashSet;

#[cfg(all(debug_assertions, feature = "verify"))]
use crate::runtime::{AnyProtocol, MethodDescription};

use objc2_encode::Encoding;

use crate::declare::{ClassBuilder, IvarType};
use crate::encode::Encode;
use crate::rc::{Allocated, Id};
use crate::runtime::{AnyClass, MethodImplementation, Sel};
use crate::runtime::{AnyObject, MessageReceiver};
use crate::{ClassType, Message, ProtocolType};

use super::{CopyOrMutCopy, Init, MaybeUnwrap, New, Other};
use crate::mutability;

/// Helper type for implementing `MethodImplementation` with a receiver of
/// `Allocated<T>`, without exposing that implementation to users.
//
// Must be private, soundness of MethodImplementation relies on this.
#[doc(hidden)]
#[repr(transparent)]
#[derive(Debug)]
pub struct IdReturnValue(pub(crate) *mut AnyObject);

// SAFETY: `IdReturnValue` is `#[repr(transparent)]`
unsafe impl Encode for IdReturnValue {
    const ENCODING: Encoding = <*mut AnyObject>::ENCODING;
}

// One could imagine a different design where we had a method like
// `fn convert_receiver()`, but that won't work in `declare_class!` since we
// can't actually modify the `self` argument (e.g. `let self = foo(self)` is
// not allowed).
//
// See `MsgSendId` and `RetainSemantics` for details on the retain semantics
// we're following here.
pub trait MessageRecieveId<Receiver, Ret> {
    fn into_return(obj: Ret) -> IdReturnValue;
}

// Receiver and return type have no correlation
impl<Receiver, Ret> MessageRecieveId<Receiver, Ret> for New
where
    Receiver: MessageReceiver,
    Ret: MaybeOptionId,
{
    #[inline]
    fn into_return(obj: Ret) -> IdReturnValue {
        obj.consumed_return()
    }
}

// Explicitly left unimplemented for now!
// impl MessageRecieveId<impl MessageReceiver, Allocated<T>> for Alloc {}

// Note: `MethodImplementation` allows for `Allocated` as the receiver, so we
// restrict it here to only be when the selector is `init`.
//
// Additionally, the receiver and return type must have the same generic
// parameter `T`.
impl<Ret, T> MessageRecieveId<Allocated<T>, Ret> for Init
where
    T: Message,
    Ret: MaybeOptionId<Input = Option<Id<T>>>,
{
    #[inline]
    fn into_return(obj: Ret) -> IdReturnValue {
        obj.consumed_return()
    }
}

// Receiver and return type have no correlation
impl<Receiver, Ret> MessageRecieveId<Receiver, Ret> for CopyOrMutCopy
where
    Receiver: MessageReceiver,
    Ret: MaybeOptionId,
{
    #[inline]
    fn into_return(obj: Ret) -> IdReturnValue {
        obj.consumed_return()
    }
}

// Receiver and return type have no correlation
impl<Receiver, Ret> MessageRecieveId<Receiver, Ret> for Other
where
    Receiver: MessageReceiver,
    Ret: MaybeOptionId,
{
    #[inline]
    fn into_return(obj: Ret) -> IdReturnValue {
        obj.autorelease_return()
    }
}

/// Helper trait for specifying an `Id<T>` or an `Option<Id<T>>`.
///
/// (Both of those are valid return types from declare_class! `#[method_id]`).
pub trait MaybeOptionId: MaybeUnwrap {
    fn consumed_return(self) -> IdReturnValue;
    fn autorelease_return(self) -> IdReturnValue;
}

impl<T: Message> MaybeOptionId for Id<T> {
    #[inline]
    fn consumed_return(self) -> IdReturnValue {
        let ptr: *mut T = Id::consume_as_ptr(self);
        IdReturnValue(ptr.cast())
    }

    #[inline]
    fn autorelease_return(self) -> IdReturnValue {
        let ptr: *mut T = Id::autorelease_return(self);
        IdReturnValue(ptr.cast())
    }
}

impl<T: Message> MaybeOptionId for Option<Id<T>> {
    #[inline]
    fn consumed_return(self) -> IdReturnValue {
        let ptr: *mut T = Id::consume_as_ptr_option(self);
        IdReturnValue(ptr.cast())
    }

    #[inline]
    fn autorelease_return(self) -> IdReturnValue {
        let ptr: *mut T = Id::autorelease_return_option(self);
        IdReturnValue(ptr.cast())
    }
}

/// Helper for ensuring that `ClassType::Mutability` is implemented correctly
/// for subclasses.
pub trait ValidSubclassMutability<T: mutability::Mutability> {}

// Root
impl ValidSubclassMutability<mutability::Immutable> for mutability::Root {}
impl ValidSubclassMutability<mutability::Mutable> for mutability::Root {}
impl<MS, IS> ValidSubclassMutability<mutability::ImmutableWithMutableSubclass<MS>>
    for mutability::Root
where
    MS: ?Sized + ClassType<Mutability = mutability::MutableWithImmutableSuperclass<IS>>,
    IS: ?Sized + ClassType<Mutability = mutability::ImmutableWithMutableSubclass<MS>>,
{
}
impl ValidSubclassMutability<mutability::InteriorMutable> for mutability::Root {}
impl ValidSubclassMutability<mutability::MainThreadOnly> for mutability::Root {}

// Immutable
impl ValidSubclassMutability<mutability::Immutable> for mutability::Immutable {}

// Mutable
impl ValidSubclassMutability<mutability::Mutable> for mutability::Mutable {}

// ImmutableWithMutableSubclass
impl<MS, IS> ValidSubclassMutability<mutability::MutableWithImmutableSuperclass<IS>>
    for mutability::ImmutableWithMutableSubclass<MS>
where
    MS: ?Sized + ClassType<Mutability = mutability::MutableWithImmutableSuperclass<IS>>,
    IS: ?Sized + ClassType<Mutability = mutability::ImmutableWithMutableSubclass<MS>>,
{
}
// Only valid when `NSCopying`/`NSMutableCopying` is not implemented!
impl<MS: ?Sized + ClassType> ValidSubclassMutability<mutability::Immutable>
    for mutability::ImmutableWithMutableSubclass<MS>
{
}

// MutableWithImmutableSuperclass
// Only valid when `NSCopying`/`NSMutableCopying` is not implemented!
impl<IS: ?Sized + ClassType> ValidSubclassMutability<mutability::Mutable>
    for mutability::MutableWithImmutableSuperclass<IS>
{
}

// InteriorMutable
impl ValidSubclassMutability<mutability::InteriorMutable> for mutability::InteriorMutable {}
impl ValidSubclassMutability<mutability::MainThreadOnly> for mutability::InteriorMutable {}

// MainThreadOnly
impl ValidSubclassMutability<mutability::MainThreadOnly> for mutability::MainThreadOnly {}

/// Ensure that:
/// 1. The type is not a root class (it's superclass implements `ClassType`,
///    and it's mutability is not `Root`), and therefore also implements basic
///    memory management methods, as required by `unsafe impl Message`.
/// 2. The mutability is valid according to the superclass' mutability.
#[inline]
pub fn assert_mutability_matches_superclass_mutability<T>()
where
    T: ?Sized + ClassType,
    T::Super: ClassType,
    T::Mutability: mutability::Mutability,
    <T::Super as ClassType>::Mutability: ValidSubclassMutability<T::Mutability>,
{
    // Noop
}

#[derive(Debug)]
pub struct ClassBuilderHelper<T: ?Sized> {
    builder: ClassBuilder,
    p: PhantomData<T>,
}

#[track_caller]
fn failed_declaring_class(name: &str) -> ! {
    panic!("could not create new class {name}. Perhaps a class with that name already exists?")
}

impl<T: ?Sized + ClassType> ClassBuilderHelper<T> {
    #[inline]
    #[track_caller]
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self
    where
        T::Super: ClassType,
    {
        let builder = match ClassBuilder::new(T::NAME, <T::Super as ClassType>::class()) {
            Some(builder) => builder,
            None => failed_declaring_class(T::NAME),
        };

        Self {
            builder,
            p: PhantomData,
        }
    }

    #[inline]
    pub fn add_protocol_methods<P>(&mut self) -> ClassProtocolMethodsBuilder<'_, T>
    where
        P: ?Sized + ProtocolType,
    {
        let protocol = P::protocol();

        if let Some(protocol) = protocol {
            self.builder.add_protocol(protocol);
        }

        #[cfg(all(debug_assertions, feature = "verify"))]
        {
            ClassProtocolMethodsBuilder {
                builder: self,
                protocol,
                required_instance_methods: protocol
                    .map(|p| p.method_descriptions(true))
                    .unwrap_or_default(),
                optional_instance_methods: protocol
                    .map(|p| p.method_descriptions(false))
                    .unwrap_or_default(),
                registered_instance_methods: HashSet::new(),
                required_class_methods: protocol
                    .map(|p| p.class_method_descriptions(true))
                    .unwrap_or_default(),
                optional_class_methods: protocol
                    .map(|p| p.class_method_descriptions(false))
                    .unwrap_or_default(),
                registered_class_methods: HashSet::new(),
            }
        }

        #[cfg(not(all(debug_assertions, feature = "verify")))]
        {
            ClassProtocolMethodsBuilder { builder: self }
        }
    }

    // Addition: This restricts to callee `T`
    #[inline]
    pub unsafe fn add_method<F>(&mut self, sel: Sel, func: F)
    where
        F: MethodImplementation<Callee = T>,
    {
        // SAFETY: Checked by caller
        unsafe { self.builder.add_method(sel, func) }
    }

    #[inline]
    pub unsafe fn add_class_method<F>(&mut self, sel: Sel, func: F)
    where
        F: MethodImplementation<Callee = AnyClass>,
    {
        // SAFETY: Checked by caller
        unsafe { self.builder.add_class_method(sel, func) }
    }

    #[inline]
    pub fn add_static_ivar<I: IvarType>(&mut self) {
        self.builder.add_static_ivar::<I>()
    }

    #[inline]
    pub fn register(self) -> &'static AnyClass {
        self.builder.register()
    }
}

/// Helper for ensuring that:
/// - Only methods on the protocol are overriden.
/// - TODO: The methods have the correct signature.
/// - All required methods are overridden.
#[derive(Debug)]
pub struct ClassProtocolMethodsBuilder<'a, T: ?Sized> {
    builder: &'a mut ClassBuilderHelper<T>,
    #[cfg(all(debug_assertions, feature = "verify"))]
    protocol: Option<&'static AnyProtocol>,
    #[cfg(all(debug_assertions, feature = "verify"))]
    required_instance_methods: Vec<MethodDescription>,
    #[cfg(all(debug_assertions, feature = "verify"))]
    optional_instance_methods: Vec<MethodDescription>,
    #[cfg(all(debug_assertions, feature = "verify"))]
    registered_instance_methods: HashSet<Sel>,
    #[cfg(all(debug_assertions, feature = "verify"))]
    required_class_methods: Vec<MethodDescription>,
    #[cfg(all(debug_assertions, feature = "verify"))]
    optional_class_methods: Vec<MethodDescription>,
    #[cfg(all(debug_assertions, feature = "verify"))]
    registered_class_methods: HashSet<Sel>,
}

impl<T: ?Sized + ClassType> ClassProtocolMethodsBuilder<'_, T> {
    // Addition: This restricts to callee `T`
    #[inline]
    pub unsafe fn add_method<F>(&mut self, sel: Sel, func: F)
    where
        F: MethodImplementation<Callee = T>,
    {
        #[cfg(all(debug_assertions, feature = "verify"))]
        if let Some(protocol) = self.protocol {
            let _types = self
                .required_instance_methods
                .iter()
                .chain(&self.optional_instance_methods)
                .find(|desc| desc.sel == sel)
                .map(|desc| desc.types)
                .unwrap_or_else(|| {
                    panic!(
                        "failed overriding protocol method -[{protocol} {sel}]: method not found"
                    )
                });
        }

        // SAFETY: Checked by caller
        unsafe { self.builder.add_method(sel, func) };

        #[cfg(all(debug_assertions, feature = "verify"))]
        if !self.registered_instance_methods.insert(sel) {
            unreachable!("already added")
        }
    }

    #[inline]
    pub unsafe fn add_class_method<F>(&mut self, sel: Sel, func: F)
    where
        F: MethodImplementation<Callee = AnyClass>,
    {
        #[cfg(all(debug_assertions, feature = "verify"))]
        if let Some(protocol) = self.protocol {
            let _types = self
                .required_class_methods
                .iter()
                .chain(&self.optional_class_methods)
                .find(|desc| desc.sel == sel)
                .map(|desc| desc.types)
                .unwrap_or_else(|| {
                    panic!(
                        "failed overriding protocol method +[{protocol} {sel}]: method not found"
                    )
                });
        }

        // SAFETY: Checked by caller
        unsafe { self.builder.add_class_method(sel, func) };

        #[cfg(all(debug_assertions, feature = "verify"))]
        if !self.registered_class_methods.insert(sel) {
            unreachable!("already added")
        }
    }

    #[cfg(all(debug_assertions, feature = "verify"))]
    pub fn finish(self) {
        let superclass = self.builder.builder.superclass();

        if let Some(protocol) = self.protocol {
            for desc in &self.required_instance_methods {
                if self.registered_instance_methods.contains(&desc.sel) {
                    continue;
                }

                // TODO: Don't do this when `NS_PROTOCOL_REQUIRES_EXPLICIT_IMPLEMENTATION`
                if superclass
                    .and_then(|superclass| superclass.instance_method(desc.sel))
                    .is_some()
                {
                    continue;
                }

                panic!(
                    "must implement required protocol method -[{protocol} {}]",
                    desc.sel
                )
            }
        }

        if let Some(protocol) = self.protocol {
            for desc in &self.required_class_methods {
                if self.registered_class_methods.contains(&desc.sel) {
                    continue;
                }

                // TODO: Don't do this when `NS_PROTOCOL_REQUIRES_EXPLICIT_IMPLEMENTATION`
                if superclass
                    .and_then(|superclass| superclass.class_method(desc.sel))
                    .is_some()
                {
                    continue;
                }

                panic!(
                    "must implement required protocol method +[{protocol} {}]",
                    desc.sel
                );
            }
        }
    }

    #[inline]
    #[cfg(not(all(debug_assertions, feature = "verify")))]
    pub fn finish(self) {}
}
