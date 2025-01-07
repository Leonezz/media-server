pub mod fill_element;

pub mod id_syn_ele {
    pub const ID_SCE: u8 = 0x0; // single_channel_element
    pub const ID_CPE: u8 = 0x1; // channel_pair_element
    pub const ID_CCE: u8 = 0x2; // coupling_channel_element
    pub const ID_LFE: u8 = 0x3; // lfe_channel_element
    pub const ID_DSE: u8 = 0x4; // data_stream_element
    pub const ID_PCE: u8 = 0x5; // program_config_element
    pub const ID_FIL: u8 = 0x6; // fill_element
    pub const ID_END: u8 = 0x7; // TERM
}

#[derive(Debug)]
pub enum Element {
    SingleChannelElement,
    ChannelPairElement,
    CouplingChannelElement,
    IFEChannelElement,
    DataStreamElement,
    ProgramConfigElement,
    FillElement,
}
#[derive(Debug)]
pub struct RawDataBlock {
    pub id: u8, // 3 bits
    pub elements: Vec<Element>,
}
