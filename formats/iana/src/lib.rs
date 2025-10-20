pub mod addrress_family;
pub mod protocol_numbers;
#[macro_export]
macro_rules! rfc_url {
    ($rfc:ident) => {
        concat!(
            "[",
            stringify!($rfc),
            "]",
            "(https://www.rfc-editor.org/rfc/",
            stringify!($rfc),
            ".html)"
        )
    };
    ($rfc:ident, $($rest:ident),+ $(,)?) => {
        concat!(
            rfc_url!($rfc),
            ", ",
            rfc_url!($($rest),+)
        )
    };
}
