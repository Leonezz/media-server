pub mod rfc8656;

#[cfg(test)]
mod test {
    use std::iter::zip;

    use stun_formats::methods::{MethodExtStatic, from_value};

    use crate::methods::rfc8656::{ALLOCATE, CHANNEL_BIND, CREATE_PERMISSION, DATA, REFRESH, SEND};

    #[test]
    fn test_from_value() {
        let registered = [
            ALLOCATE::STATIC_VALUE,
            CHANNEL_BIND::STATIC_VALUE,
            CREATE_PERMISSION::STATIC_VALUE,
            DATA::STATIC_VALUE,
            REFRESH::STATIC_VALUE,
            SEND::STATIC_VALUE,
        ];
        let names = [
            ALLOCATE::STATIC_NAME,
            CHANNEL_BIND::STATIC_NAME,
            CREATE_PERMISSION::STATIC_NAME,
            DATA::STATIC_NAME,
            REFRESH::STATIC_NAME,
            SEND::STATIC_NAME,
        ];
        for (value, name) in zip(registered, names) {
            let m = from_value(value);
            assert!(m.is_some_and(|item| item.name() == name))
        }
    }
}
