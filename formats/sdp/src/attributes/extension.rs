use crate::{attributes::SDPAttribute, errors::SDPError};

pub trait SdpAttributeExtension: Sized {
    fn attr_name() -> &'static str;
    fn value(&self) -> Option<String>;
    fn match_attribute(attr: &SDPAttribute) -> bool {
        attr.name() == Self::attr_name()
    }
    fn try_from_attr(attr: &SDPAttribute) -> Result<Self, SDPError>;
    fn into_attr(self) -> SDPAttribute;
    fn into_value(self) -> Option<String> {
        self.value()
    }
}
