use utils::traits::{
    dynamic_sized_packet::DynamicSizedBitsPacket, fixed_packet::FixedBitwisePacket,
};

use super::{DataWithLength, Symbol};

#[derive(Debug, Clone)]
pub struct EventTime {
    pub use_if_late: bool, // 1 bit
    pub time: f32,         // 32 bits
}

impl FixedBitwisePacket for EventTime {
    fn bits_count() -> usize {
        1 + // use_if_late
        32 // time
    }
}

#[derive(Debug, Clone)]
pub struct InstrEvent {
    #[allow(unused)]
    pub(crate) has_label: bool, // 1 bit,
    pub label: Option<Symbol>,
    pub iname_sym: Symbol,
    pub dur: f32,     // 32 bits
    pub num_pf: u8,   // 8 bits
    pub pf: Vec<f32>, // f32 pf[num_pf]
}

impl DynamicSizedBitsPacket for InstrEvent {
    fn get_packet_bits_count(&self) -> usize {
        1 + // has_label
        self.label.map_or(0, |_| 16) +
        16 + // iname_sym
        32 + // dur
        8 + // num_pf
        self.pf.len() * 32
    }
}

#[derive(Debug, Clone)]
pub struct ControlEvent {
    #[allow(unused)]
    pub(crate) has_label: bool, // 1 bit
    pub label: Option<Symbol>,
    pub varsym: Symbol,
    pub value: f32, // 32 bits
}

impl DynamicSizedBitsPacket for ControlEvent {
    fn get_packet_bits_count(&self) -> usize {
        1 + // has_label
        self.label.map_or(0, |_| 16) +
        16 + // varsym
        32 // value
    }
}

#[derive(Debug, Clone)]
pub struct TableEventContent {
    pub tgen: u8, // what the fuck?
    #[allow(unused)]
    pub(crate) refers_to_sample: bool, // 1 bit
    pub table_sym: Option<Symbol>,
    pub num_pf: u16, // 16 bits
    // if tgen == 0x7D {
    pub size: Option<f32>,       // 32 bits
    pub ft: Option<Vec<Symbol>>, // symbol ft[num_pf - 1]
    // } else {
    pub pf: Option<Vec<f32>>, // f32 pf[num_pf]
                              // }
}

impl DynamicSizedBitsPacket for TableEventContent {
    fn get_packet_bits_count(&self) -> usize {
        8 + // token
        1 + // refers_to_sample
        self.table_sym.map_or(0, |_| 16) +
        16 + // num_pf
        self.size.map_or(0, |_|32) +
        self.ft.as_ref().map_or(0, |item| item.len() * 16) +
        self.pf.as_ref().map_or(0, |item| item.len() * 32)
    }
}

#[derive(Debug, Clone)]
pub struct TableEvent {
    pub tname: Symbol,
    pub destroy: bool,
    // if !destroy {
    pub content: Option<TableEventContent>, //}
}

impl DynamicSizedBitsPacket for TableEvent {
    fn get_packet_bits_count(&self) -> usize {
        16 + // tname
        1 + // destroy
        self.content.as_ref().map_or(0, |item| item.get_packet_bits_count())
    }
}

#[derive(Debug, Clone)]
pub enum Event {
    Inst(InstrEvent),
    Control(ControlEvent),
    Table(TableEvent),
    End(/*fixed at nothing */),
    Tempo {
        tempo: f32, // 32 bits
    },
}

impl Event {
    pub fn get_type(&self) -> u8 {
        match self {
            Self::Inst(_) => 0b000,
            Self::Control(_) => 0b001,
            Self::Table(_) => 0b010,
            Self::End() => 0b100,
            Self::Tempo { .. } => 0b101,
        }
    }
}

impl DynamicSizedBitsPacket for Event {
    fn get_packet_bits_count(&self) -> usize {
        match self {
            Self::Inst(instr_event) => instr_event.get_packet_bits_count(),
            Self::Control(control_event) => control_event.get_packet_bits_count(),
            Self::Table(table_event) => table_event.get_packet_bits_count(),
            Self::End() => 0,
            Self::Tempo { .. } => 32,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ScoreLine {
    #[allow(unused)]
    pub(crate) has_time: bool, // 1 bit
    pub event_time: Option<EventTime>,
    pub high_priority: bool, // 1 bit
    #[allow(unused)]
    pub(crate) event_type: u8, // 3 bits
    pub event: Event,
}

impl DynamicSizedBitsPacket for ScoreLine {
    fn get_packet_bits_count(&self) -> usize {
        1 + // has_time
        self.event_time.as_ref().map_or(0, |_| EventTime::bits_count()) +
        1 + // high_priority
        3 + // type
        self.event.get_packet_bits_count()
    }
}

pub type ScoreFile = DataWithLength<u32, ScoreLine>;
