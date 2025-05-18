pub mod reader;
pub mod writer;

#[derive(Debug, Clone)]
pub struct AuxFormatIdcRelated {
    pub bit_depth_aux_minus8: u8,     // ue(v), in [0, 4]
    pub alpha_incr_flag: bool,        // u(1)
    pub alpha_opaque_value: u16,      // u(v), v = bit_depth_aux_minus8 + 9
    pub alpha_transparent_value: u16, // u(v), v = bit_depth_aux_minus8 + 9
}

/// @see: Recommendation  ITU-T H.264 (V15) (08/2024)   – Coding of moving video
/// Section 7.3.2.1.2 Sequence parameter set extension RBSP syntax
#[derive(Debug, Clone)]
pub struct SpsExt {
    pub seq_parameter_set_id: u64, // ue(v)
    pub aux_format_idc: u8,        // ue(v), in [0, 3]
    /// if( aux_format_idc != 0 ) {
    pub aux_format_idc_related: Option<AuxFormatIdcRelated>,
    /// }
    pub additional_extension_flag: bool, // u(1)
}
