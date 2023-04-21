data! {
    // SAFETY: `new` or `initWithObjects:` may choose to deduplicate arrays,
    // and returning mutable references to those would be unsound - hence
    // `NSArray` cannot be mutable.
    class NSArray: ImmutableWithMutableSubclass<Foundation::NSMutableArray> {
        unsafe -count;
    }

    class NSMutableArray: MutableWithImmutableSuperclass<Foundation::NSArray> {
        unsafe mut -removeAllObjects;
        mut -addObject;
        mut -insertObject_atIndex;
        mut -replaceObjectAtIndex_withObject;
        mut -removeObjectAtIndex;
        mut -removeLastObject;
        mut -sortUsingFunction_context;
    }

    class NSString: ImmutableWithMutableSubclass<Foundation::NSMutableString> {
        unsafe -init;
        unsafe -compare;
        unsafe -hasPrefix;
        unsafe -hasSuffix;
        // The other string is non-null, and won't be retained
        unsafe -stringByAppendingString;
        unsafe -stringByAppendingPathComponent;
        // Assuming `NSStringEncoding` can be made safe
        unsafe -lengthOfBytesUsingEncoding;
        unsafe -length;
        // Safe to call, but the returned pointer may not be safe to use
        unsafe -UTF8String;
        unsafe -initWithString;
        unsafe +stringWithString;
    }

    class NSMutableString: MutableWithImmutableSuperclass<Foundation::NSString> {
        unsafe -initWithCapacity;
        unsafe +stringWithCapacity;
        unsafe -initWithString;
        unsafe +stringWithString;
        unsafe mut -appendString;
        unsafe mut -setString;
    }

    // Allowed to be just `Immutable` since we've removed the `NSCopying` and
    // `NSMutableCopying` impls from these for now (they'd return the wrong
    // type).
    class NSSimpleCString: Immutable {}
    class NSConstantString: Immutable {}

    class NSAttributedString: ImmutableWithMutableSubclass<Foundation::NSMutableAttributedString> {
        unsafe -initWithString;
        unsafe -initWithAttributedString;
        unsafe -string;
        unsafe -length;
    }

    class NSMutableAttributedString: MutableWithImmutableSuperclass<Foundation::NSAttributedString> {
        unsafe -initWithString;
        unsafe -initWithAttributedString;
        unsafe mut -setAttributedString;
    }

    class NSBundle {
        unsafe +mainBundle;
        unsafe -infoDictionary;
    }

    class NSData: ImmutableWithMutableSubclass<Foundation::NSMutableData> {
        unsafe -initWithData;
        unsafe +dataWithData;
        unsafe -length;
        unsafe -bytes;
    }

    class NSMutableData: MutableWithImmutableSuperclass<Foundation::NSData> {
        unsafe +dataWithData;
        unsafe -initWithCapacity;
        unsafe +dataWithCapacity;
        unsafe mut -setLength;
        unsafe mut -mutableBytes;
    }

    // Allowed to be just `Mutable` since we've removed the `NSCopying` and
    // `NSMutableCopying` impls from this for now (since they'd return the
    // wrong type).
    class NSPurgeableData: Mutable {}

    class NSDictionary: ImmutableWithMutableSubclass<Foundation::NSMutableDictionary> {
        unsafe -count;
    }

    class NSMutableDictionary: MutableWithImmutableSuperclass<Foundation::NSDictionary> {
        unsafe mut -removeObjectForKey;
        unsafe mut -removeAllObjects;
        mut -setDictionary;
    }

    class NSError {
        unsafe -domain;
        unsafe -code;
        unsafe -userInfo;
        unsafe -localizedDescription;
    }

    class NSException {
        unsafe -name;
        unsafe -reason;
        unsafe -userInfo;
    }

    class NSLock {
        unsafe -name;
        unsafe -setName;
    }

    class NSValue {
        unsafe -objCType;
        unsafe -isEqualToValue;
    }

    class NSUUID {
        unsafe +UUID;
        unsafe -init;
        unsafe -initWithUUIDString;
        unsafe -UUIDString;
    }

    class NSThread {
        unsafe +currentThread;
        unsafe +mainThread;
        unsafe -name;
        unsafe +isMultiThreaded;
        unsafe -isMainThread;
        unsafe +isMainThread;
    }

    class NSProcessInfo {
        unsafe +processInfo;
        unsafe -processName;
        unsafe -operatingSystemVersion;
    }

    class NSSet: ImmutableWithMutableSubclass<Foundation::NSMutableSet> {
        unsafe -count;
    }

    class NSMutableSet: MutableWithImmutableSuperclass<Foundation::NSSet> {
        unsafe mut -removeAllObjects;
        mut -addObject;
    }

    class NSCharacterSet: ImmutableWithMutableSubclass<Foundation::NSMutableCharacterSet> {}
    class NSMutableCharacterSet: MutableWithImmutableSuperclass<Foundation::NSCharacterSet> {}

    class NSOrderedSet: ImmutableWithMutableSubclass<Foundation::NSMutableOrderedSet> {}
    class NSMutableOrderedSet: MutableWithImmutableSuperclass<Foundation::NSOrderedSet> {}

    class NSIndexSet: ImmutableWithMutableSubclass<Foundation::NSMutableIndexSet> {}
    class NSMutableIndexSet: MutableWithImmutableSuperclass<Foundation::NSIndexSet> {}

    class NSNumber {
        unsafe -initWithChar;
        unsafe -initWithUnsignedChar;
        unsafe -initWithShort;
        unsafe -initWithUnsignedShort;
        unsafe -initWithInt;
        unsafe -initWithUnsignedInt;
        unsafe -initWithLong;
        unsafe -initWithUnsignedLong;
        unsafe -initWithLongLong;
        unsafe -initWithUnsignedLongLong;
        unsafe -initWithFloat;
        unsafe -initWithDouble;
        unsafe -initWithBool;
        unsafe -initWithInteger;
        unsafe -initWithUnsignedInteger;
        unsafe +numberWithChar;
        unsafe +numberWithUnsignedChar;
        unsafe +numberWithShort;
        unsafe +numberWithUnsignedShort;
        unsafe +numberWithInt;
        unsafe +numberWithUnsignedInt;
        unsafe +numberWithLong;
        unsafe +numberWithUnsignedLong;
        unsafe +numberWithLongLong;
        unsafe +numberWithUnsignedLongLong;
        unsafe +numberWithFloat;
        unsafe +numberWithDouble;
        unsafe +numberWithBool;
        unsafe +numberWithInteger;
        unsafe +numberWithUnsignedInteger;
        unsafe -compare;
        unsafe -isEqualToNumber;
        unsafe -charValue;
        unsafe -unsignedCharValue;
        unsafe -shortValue;
        unsafe -unsignedShortValue;
        unsafe -intValue;
        unsafe -unsignedIntValue;
        unsafe -longValue;
        unsafe -unsignedLongValue;
        unsafe -longLongValue;
        unsafe -unsignedLongLongValue;
        unsafe -floatValue;
        unsafe -doubleValue;
        unsafe -boolValue;
        unsafe -integerValue;
        unsafe -unsignedIntegerValue;
        unsafe -stringValue;
    }

    class NSURLRequest: ImmutableWithMutableSubclass<Foundation::NSMutableURLRequest> {}
    class NSMutableURLRequest: MutableWithImmutableSuperclass<Foundation::NSURLRequest> {}
}
