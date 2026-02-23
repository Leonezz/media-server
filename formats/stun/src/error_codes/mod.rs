use std::fmt::Debug;

pub mod rfc8489;

pub trait ErrorCodeExtStatic: Default {
    const STATIC_CODE: u16;
    const STATIC_NAME: &'static str;
    const STATIC_REASON: &'static str;
}

pub trait ErrorCodeExtDynamic: Debug + Send + Sync {
    fn code(&self) -> u16;
    fn name(&self) -> &str;
    fn reason(&self) -> &str;
}

pub struct ErrorCodeExtEntry {
    pub code: u16,
    pub factory: fn() -> Box<dyn ErrorCodeExtDynamic>,
    pub new_with_reason: fn(&str) -> Box<dyn ErrorCodeExtDynamic>,
}

inventory::collect!(ErrorCodeExtEntry);

pub fn from_code(code: u16) -> Option<Box<dyn ErrorCodeExtDynamic>> {
    for entry in inventory::iter::<ErrorCodeExtEntry> {
        if entry.code == code {
            return Some((entry.factory)());
        }
    }
    None
}

pub fn from_code_with_reason(code: u16, reason: &str) -> Option<Box<dyn ErrorCodeExtDynamic>> {
    for entry in inventory::iter::<ErrorCodeExtEntry> {
        if entry.code == code {
            return Some((entry.new_with_reason)(reason));
        }
    }
    None
}

#[macro_export]
macro_rules! define_error_code {
    ($code: tt, $id: ident, $reason: expr) => {
        #[allow(non_camel_case_types)]
        pub struct $id(String);

        impl Default for $id {
            fn default() -> Self {
                Self($reason.to_string())
            }
        }

        impl $id {
            pub fn new() -> Self {
                Self::default()
            }

            pub fn new_with_reason<S: Into<String>>(reason: S) -> Self {
                Self(reason.into())
            }
        }

        impl ErrorCodeExtStatic for $id {
            const STATIC_CODE: u16 = $code;
            const STATIC_NAME: &'static str = stringify!($id);
            const STATIC_REASON: &'static str = $reason;
        }

        impl ErrorCodeExtDynamic for $id {
            fn code(&self) -> u16 {
                $code
            }

            fn name(&self) -> &str {
                stringify!($id)
            }

            fn reason(&self) -> &str {
                self.0.as_str()
            }
        }

        inventory::submit! {
            $crate::error_codes::ErrorCodeExtEntry {
                code: $code,
                factory: || Box::new($id::new()),
                new_with_reason: |str| Box::new($id::new_with_reason(str))
            }
        }

        impl std::fmt::Debug for $id {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(
                    f,
                    "{}({}): {}",
                    Self::STATIC_NAME,
                    Self::STATIC_CODE,
                    self.reason()
                )
            }
        }
    };
}
