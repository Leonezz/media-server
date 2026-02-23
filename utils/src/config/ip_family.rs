use clap::builder::TypedValueParser;

pub fn ip_family_parser(
    allowed: &'static [&'static str],
) -> impl clap::builder::TypedValueParser<Value = iana_formats::addrress_family::AddressFamily> {
    clap::builder::PossibleValuesParser::new(allowed).map(|s| {
        s.parse::<iana_formats::addrress_family::AddressFamily>()
            .expect("validated")
    })
}
