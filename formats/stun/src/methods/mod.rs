use std::fmt::Debug;

use crate::MessageChecker;

pub mod rfc8489;
pub trait MethodExtStatic {
    const STATIC_VALUE: u16;
    const STATIC_NAME: &'static str;
}

pub trait MethodExtDynamicInner {
    fn value(&self) -> u16;
    fn name(&self) -> &str;
}

pub trait MethodExtDynamic: MethodExtDynamicInner + MessageChecker + Debug + Send + Sync {}

pub trait CloneableMethodExt: MethodExtDynamic {
    fn clone_box(&self) -> Box<dyn CloneableMethodExt>;
}

impl<T> CloneableMethodExt for T
where
    T: 'static + Clone + MethodExtDynamic,
{
    fn clone_box(&self) -> Box<dyn CloneableMethodExt> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn CloneableMethodExt> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

pub struct MethodExtEntry {
    pub value: u16,
    pub factory: fn() -> Box<dyn MethodExtDynamic>,
}

inventory::collect!(MethodExtEntry);
pub fn from_value(value: u16) -> Option<Box<dyn MethodExtDynamic>> {
    for entry in inventory::iter::<MethodExtEntry> {
        if entry.value == value {
            return Some((entry.factory)());
        }
    }
    None
}

#[macro_export]
macro_rules! define_method {
    ($value: tt, $id: ident, $name: expr) => {
        #[allow(non_camel_case_types)]
        #[derive(Clone)]
        pub struct $id;

        impl $id {
            pub fn new() -> Self {
                Self::default()
            }
        }

        impl Default for $id {
            fn default() -> Self {
                Self {}
            }
        }

        impl MethodExtStatic for $id {
            const STATIC_VALUE: u16 = $value;
            const STATIC_NAME: &'static str = $name;
        }

        impl MethodExtDynamicInner for $id {
            fn value(&self) -> u16 {
                $value
            }
            fn name(&self) -> &str {
                $name
            }
        }

        inventory::submit! {
            $crate::methods::MethodExtEntry {
                value: $value,
                factory: || Box::new($id::new()),
            }
        }

        impl std::fmt::Debug for $id {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}({})", Self::STATIC_NAME, Self::STATIC_VALUE)
            }
        }
    };
}

#[cfg(test)]
mod test {
    use crate::{errors::StunMessageResult, methods};

    use super::*;

    // define a test method and register it via inventory so from_value can find it
    define_method!(0x0FFFu16, test_method_fff, "TEST_METHOD");

    #[allow(unused_variables)]
    impl MessageChecker for test_method_fff {
        fn allowed_in(&self, message_class: crate::header::MessageClass) -> bool {
            true
        }
        fn check_error_response(&self, message: &crate::message::Message) -> StunMessageResult<()> {
            Ok(())
        }
        fn check_indication(&self, message: &crate::message::Message) -> StunMessageResult<()> {
            Ok(())
        }
        fn check_request(&self, message: &crate::message::Message) -> StunMessageResult<()> {
            Ok(())
        }
        fn check_success_response(
            &self,
            message: &crate::message::Message,
        ) -> StunMessageResult<()> {
            Ok(())
        }
    }

    impl MethodExtDynamic for test_method_fff {}

    fn test_method_factory() -> Box<dyn MethodExtDynamic> {
        Box::new(test_method_fff::default())
    }

    inventory::submit! {
        MethodExtEntry {
            value: test_method_fff::STATIC_VALUE,
            factory: test_method_factory,
        }
    }

    #[test]
    fn test_from_value_none() {
        // pick a value we did not register
        assert!(from_value(0xDEAD).is_none());
    }

    #[test]
    fn test_register_and_retrieve_method() {
        let m =
            from_value(test_method_fff::STATIC_VALUE).expect("registered method should be found");
        assert_eq!(m.value(), test_method_fff::STATIC_VALUE);
        assert_eq!(m.name(), test_method_fff::STATIC_NAME);
        assert_eq!(
            format!("{:?}", m),
            format!(
                "{}({})",
                test_method_fff::STATIC_NAME,
                test_method_fff::STATIC_VALUE
            )
        );
    }

    #[test]
    fn test_from_value() {
        let m = from_value(methods::rfc8489::BINDING::STATIC_VALUE);
        assert!(m.is_some_and(|item| item.name() == methods::rfc8489::BINDING::STATIC_NAME))
    }
}
