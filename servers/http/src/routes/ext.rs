use rocket::request::FromParam;

macro_rules! generate_from_param_ext {
    ($struct_name: ident, $ext: expr) => {
        pub(crate) struct $struct_name<'a>(pub &'a str);
        impl<'a> FromParam<'a> for $struct_name<'a> {
            type Error = ();

            fn from_param(param: &'a str) -> Result<Self, Self::Error> {
                match param.strip_suffix($ext) {
                    None => Err(()),
                    Some(x) => FromParam::from_param(x)
                        .map(|x| $struct_name(x))
                        .map_err(|_| ()),
                }
            }
        }
    };
}

generate_from_param_ext!(FlvStreamName, ".flv");
// generate_from_param_ext!(HlsStreamName, ".hls");
