//! 固定Keychain itemだけを追加・読取・回収する。Authority、UI、Auditを所有しない。
#![deny(unsafe_op_in_unsafe_fn)]

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Invalid,
    Duplicate,
    Missing,
    Unavailable,
    IdentityRequired,
}

#[derive(Clone, Copy)]
pub enum Part {
    Key,
    Ciphertext,
}

pub struct Store {
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    service: String,
}
impl Store {
    pub fn new(namespace: &str) -> Result<Self, Error> {
        if !hex(namespace, 64) {
            return Err(Error::Invalid);
        }
        Ok(Self {
            service: format!("D4Pocket.credentials.v1.{namespace}"),
        })
    }
    #[cfg_attr(not(any(target_os = "macos", test)), allow(dead_code))]
    fn account(&self, id: &str, part: Part) -> Result<String, Error> {
        if !hex(id, 32) {
            return Err(Error::Invalid);
        }
        Ok(format!(
            "{id}.{}",
            match part {
                Part::Key => "key",
                Part::Ciphertext => "ciphertext",
            }
        ))
    }
}
fn hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

#[cfg(target_os = "macos")]
mod os {
    use super::*;
    use core_foundation::{
        base::{CFType, TCFType},
        boolean::CFBoolean,
        data::CFData,
        dictionary::CFDictionary,
        string::CFString,
    };
    use core_foundation_sys::{
        base::{CFGetTypeID, CFRelease, CFTypeRef},
        data::{CFDataGetTypeID, CFDataRef},
        dictionary::CFDictionaryRef,
        string::CFStringRef,
    };
    use std::ptr;
    use zeroize::Zeroizing;

    #[link(name = "Security", kind = "framework")]
    extern "C" {
        static kSecClass: CFStringRef;
        static kSecClassGenericPassword: CFStringRef;
        static kSecAttrService: CFStringRef;
        static kSecAttrAccount: CFStringRef;
        static kSecAttrAccessible: CFStringRef;
        static kSecAttrAccessibleWhenUnlockedThisDeviceOnly: CFStringRef;
        static kSecAttrSynchronizable: CFStringRef;
        static kSecUseDataProtectionKeychain: CFStringRef;
        static kSecUseAuthenticationUI: CFStringRef;
        static kSecUseAuthenticationUIFail: CFStringRef;
        static kSecValueData: CFStringRef;
        static kSecReturnData: CFStringRef;
        static kSecReturnAttributes: CFStringRef;
        static kSecMatchLimit: CFStringRef;
        static kSecMatchLimitOne: CFStringRef;
        fn SecItemAdd(query: CFDictionaryRef, result: *mut CFTypeRef) -> i32;
        fn SecItemCopyMatching(query: CFDictionaryRef, result: *mut CFTypeRef) -> i32;
        fn SecItemDelete(query: CFDictionaryRef) -> i32;
    }
    fn status(code: i32) -> Result<(), Error> {
        match code {
            0 => Ok(()),
            -25299 => Err(Error::Duplicate),
            -25300 => Err(Error::Missing),
            -34018 => Err(Error::IdentityRequired),
            _ => Err(Error::Unavailable),
        }
    }
    fn symbol(value: CFStringRef) -> CFType {
        // SAFETY: Security.frameworkの固定生存期間CFString定数。借用をretainして所有する。
        unsafe { CFType::wrap_under_get_rule(value.cast()) }
    }
    impl Store {
        fn pairs(&self, id: &str, part: Part) -> Result<Vec<(CFType, CFType)>, Error> {
            let account = self.account(id, part)?;
            // SAFETY: 固定Security定数だけを読み、queryは各同期呼出し中保持する。
            Ok(unsafe {
                vec![
                    (symbol(kSecClass), symbol(kSecClassGenericPassword)),
                    (
                        symbol(kSecAttrService),
                        CFString::new(&self.service).as_CFType(),
                    ),
                    (symbol(kSecAttrAccount), CFString::new(&account).as_CFType()),
                    (
                        symbol(kSecAttrSynchronizable),
                        CFBoolean::false_value().as_CFType(),
                    ),
                    (
                        symbol(kSecUseDataProtectionKeychain),
                        CFBoolean::true_value().as_CFType(),
                    ),
                    (
                        symbol(kSecUseAuthenticationUI),
                        symbol(kSecUseAuthenticationUIFail),
                    ),
                ]
            })
        }
        pub fn add(&self, id: &str, part: Part, bytes: &[u8]) -> Result<(), Error> {
            if bytes.is_empty()
                || bytes.len() > 131072
                || (matches!(part, Part::Key) && bytes.len() != 32)
            {
                return Err(Error::Invalid);
            }
            let mut pairs = self.pairs(id, part)?;
            // SAFETY: 不変のOS定数。CFDataは同期呼出しまで生存し、OSが自身の値を保管する。
            unsafe {
                pairs.push((
                    symbol(kSecAttrAccessible),
                    symbol(kSecAttrAccessibleWhenUnlockedThisDeviceOnly),
                ));
                pairs.push((
                    symbol(kSecValueData),
                    CFData::from_buffer(bytes).as_CFType(),
                ));
                let query = CFDictionary::from_CFType_pairs(&pairs);
                status(SecItemAdd(query.as_concrete_TypeRef(), ptr::null_mut()))
            }
        }
        pub fn read(&self, id: &str, part: Part) -> Result<Zeroizing<Vec<u8>>, Error> {
            let mut pairs = self.pairs(id, part)?;
            // SAFETY: queryは同期呼出し中有効、create-ruleの結果は一回だけ解放する。
            unsafe {
                pairs.push((symbol(kSecMatchLimit), symbol(kSecMatchLimitOne)));
                pairs.push((symbol(kSecReturnData), CFBoolean::true_value().as_CFType()));
                let query = CFDictionary::from_CFType_pairs(&pairs);
                let mut output = ptr::null();
                status(SecItemCopyMatching(
                    query.as_concrete_TypeRef(),
                    &mut output,
                ))?;
                if output.is_null() {
                    return Err(Error::Unavailable);
                }
                if CFGetTypeID(output) != CFDataGetTypeID() {
                    CFRelease(output);
                    return Err(Error::Unavailable);
                }
                let data = CFData::wrap_under_create_rule(output as CFDataRef);
                let bytes = data.bytes();
                if bytes.is_empty()
                    || bytes.len() > 131072
                    || (matches!(part, Part::Key) && bytes.len() != 32)
                {
                    return Err(Error::Unavailable);
                }
                Ok(Zeroizing::new(bytes.to_vec()))
            }
        }
        pub fn exists(&self, id: &str, part: Part) -> Result<bool, Error> {
            let mut pairs = self.pairs(id, part)?;
            // SAFETY: 属性だけを返す固定query。秘密値を読まず、所有結果を一回解放する。
            unsafe {
                pairs.push((symbol(kSecMatchLimit), symbol(kSecMatchLimitOne)));
                pairs.push((
                    symbol(kSecReturnAttributes),
                    CFBoolean::true_value().as_CFType(),
                ));
                let query = CFDictionary::from_CFType_pairs(&pairs);
                let mut output = ptr::null();
                let code = SecItemCopyMatching(query.as_concrete_TypeRef(), &mut output);
                if !output.is_null() {
                    CFRelease(output);
                }
                match status(code) {
                    Ok(()) => Ok(true),
                    Err(Error::Missing) => Ok(false),
                    Err(e) => Err(e),
                }
            }
        }
        /// exact service/account/partだけ。呼出し側の対象確認・Audit・回収責任を代替しない。
        pub fn delete(&self, id: &str, part: Part) -> Result<(), Error> {
            let query = CFDictionary::from_CFType_pairs(&self.pairs(id, part)?);
            // SAFETY: 同期呼出し中queryを保持。固定名前空間の完全一致対象だけを削除する。
            unsafe { status(SecItemDelete(query.as_concrete_TypeRef())) }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn 名前空間と対象を固定hex以外へ広げない() {
        assert!(matches!(Store::new("foreign"), Err(Error::Invalid)));
        let store = Store::new(&"a".repeat(64)).unwrap();
        assert!(matches!(
            store.account("../other", Part::Key),
            Err(Error::Invalid)
        ));
        assert_ne!(
            store.account(&"b".repeat(32), Part::Key).unwrap(),
            store.account(&"b".repeat(32), Part::Ciphertext).unwrap()
        );
    }
}
