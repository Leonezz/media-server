use clap::builder::TypedValueParser;

pub fn protocol_parser(
    allowed: &'static [&'static str],
) -> impl clap::builder::TypedValueParser<Value = iana_formats::protocol_numbers::Protocol> {
    clap::builder::PossibleValuesParser::new(allowed).map(|s| {
        s.parse::<iana_formats::protocol_numbers::Protocol>()
            .expect("validated")
    })
}
