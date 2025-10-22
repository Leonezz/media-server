/// https://www.iana.org/assignments/protocol-numbers/protocol-numbers.xhtml
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Protocol(#[allow(unused)] u8);
pub trait ProtocolNumberStatic {
    const DECIMAL: u8;
    const PROTOCOL: Protocol;
    const KEYWORD: &'static str;
    const PROTOCOL_NAME: &'static str;
    const IPV6_EXTENSION_HEADER: bool;
    const REFERENCE: &'static str;
    const DEPRECATED: bool;
}
pub trait ProtocolNumberDynamic {
    fn decimal(&self) -> u8;
    fn protocol(&self) -> Protocol;
    fn keyword(&self) -> &'static str;
    fn protocol_name(&self) -> &'static str;
    fn ipv6_extension_header(&self) -> bool;
    fn reference(&self) -> &'static str;
    fn deprecated(&self) -> bool;
}

struct ProtocolEntry {
    number: u8,
    factory: fn() -> Box<dyn ProtocolNumberDynamic>,
}

inventory::collect!(ProtocolEntry);

pub fn from_number(number: u8) -> Option<Box<dyn ProtocolNumberDynamic>> {
    for entry in inventory::iter::<ProtocolEntry> {
        if entry.number == number {
            return Some((entry.factory)());
        }
    }
    None
}

pub fn from_protocol(protocol: Protocol) -> Option<Box<dyn ProtocolNumberDynamic>> {
    from_number(protocol.0)
}

macro_rules! define_protocol_number {
    ($number:tt, $name:tt, $deprecated: tt, $protocol:tt, $ipv6:tt, $reference: expr) => {
        #[allow(non_camel_case_types)]
        pub struct $name;
        impl ProtocolNumberStatic for $name {
            const DECIMAL: u8 = $number;
            const PROTOCOL: Protocol = Protocol($number);
            const KEYWORD: &'static str = stringify!($name);
            const PROTOCOL_NAME: &'static str = $protocol;
            const IPV6_EXTENSION_HEADER: bool = $ipv6;
            const REFERENCE: &str = $reference;
            const DEPRECATED: bool = $deprecated;
        }
        impl ProtocolNumberDynamic for $name {
            fn decimal(&self) -> u8 {
                Self::DECIMAL
            }
            fn protocol(&self) -> Protocol {
                Self::PROTOCOL
            }
            fn keyword(&self) -> &'static str {
                Self::KEYWORD
            }
            fn protocol_name(&self) -> &'static str {
                Self::PROTOCOL_NAME
            }
            fn ipv6_extension_header(&self) -> bool {
                Self::IPV6_EXTENSION_HEADER
            }
            fn reference(&self) -> &'static str {
                Self::REFERENCE
            }
            fn deprecated(&self) -> bool {
                Self::DEPRECATED
            }
        }
        inventory::submit! {
            ProtocolEntry {
                number: $number,
                factory: || Box::new($name{})
            }
        }
    };
}

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

define_protocol_number!(
    0,
    HOPOPT,
    false,
    "IPv6 Hop-by-Hop Option",
    true,
    rfc_url!(rfc8200)
);
define_protocol_number!(
    1,
    ICMP,
    false,
    "Internet Control Message",
    false,
    rfc_url!(rfc792)
);
define_protocol_number!(
    2,
    IGMP,
    false,
    "Internet Group Management",
    false,
    rfc_url!(rfc1112)
);
define_protocol_number!(3, GGP, false, "Gateway-to-Gateway", false, rfc_url!(rfc823));
define_protocol_number!(
    4,
    IPv4,
    false,
    "IPv4 encapsulation",
    false,
    rfc_url!(rfc2003)
);
define_protocol_number!(5, ST, false, "Stream", false, rfc_url!(rfc1190, rfc1819));
define_protocol_number!(
    6,
    TCP,
    false,
    "Transmission Control",
    false,
    rfc_url!(rfc9293)
);
define_protocol_number!(
    7,
    CBT,
    false,
    "CBT",
    false,
    "[Tony Ballardie](mailto:A.Ballardie&cs.ucl.ac.uk)"
);
define_protocol_number!(
    8,
    EGP,
    false,
    "Exterior Gateway Protocol",
    false,
    concat!(
        rfc_url!(rfc888),
        ",",
        "[David Mills](mailto:Mills&huey.udel.edu)"
    )
);
define_protocol_number!(
    9,
    IGP,
    false,
    "any private interior gateway (used by Cisco for their IGRP)",
    false,
    "[Internet Assigned Numbers Authority](mailto:iana&iana.org)"
);
define_protocol_number!(
    10,
    BBN_RCC_MON,
    false,
    "BBN RCC Monitoring",
    false,
    "[Steve Chipman](mailto:Chipman&f.bbn.com)"
);
define_protocol_number!(
    11,
    NVP_II,
    false,
    "Network Voice Protocol",
    false,
    concat!(
        rfc_url!(rfc741),
        ",",
        "[Steve Casner](mailto:casner&isi.edu)"
    )
);
define_protocol_number!(
    12,
    PUP,
    false,
    "PUP",
    false,
    "[Boggs, D., J. Shoch, E. Taft, and R. Metcalfe, \"PUP: An Internetwork Architecture\", XEROX Palo Alto Research Center, CSL-79-10, July 1979; also in IEEE Transactions on Communication, Volume COM-28, Number 4, April 1980.][[XEROX]]"
);
define_protocol_number!(
    13,
    ARGUS,
    true,
    "ARGUS",
    false,
    "[Robert W. Scheifler](mailto:rscheifler&comcast.net)"
);
define_protocol_number!(
    14,
    EMCON,
    false,
    "EMCON",
    false,
    "[Bich Nguyen](mailto:bitnguyen&gmail.com)"
);
define_protocol_number!(
    15,
    XNET,
    false,
    "Cross Net Debugger",
    false,
    "[Haverty, J., \"XNET Formats for Internet Protocol Version 4\", IEN 158, October 1980.][Jack Haverty](mailto:jhaverty&oracle.com)"
);
define_protocol_number!(
    16,
    CHAOS,
    false,
    "Chaos",
    false,
    "[J. Noel Chiappa](mailto:JNC&xx.lcs.mit.edu)"
);
define_protocol_number!(
    17,
    UDP,
    false,
    "User Datagram",
    false,
    concat!(rfc_url!(rfc768), ",", "[Jon Postel](mailto:postel&isi.edu)")
);
define_protocol_number!(
    18,
    MUX,
    false,
    "Multiplexing",
    false,
    "[Cohen, D. and J. Postel, \"Multiplexing Protocol\", IEN 90, USC/Information Sciences Institute, May 1979.][Jon Postel](mailto:postel&isi.edu)"
);
define_protocol_number!(
    19,
    DCN_MEAS,
    false,
    "DCN Measurement Subsystems",
    false,
    "[David Mills](mailto:Mills&huey.udel.edu)"
);
define_protocol_number!(
    20,
    HMP,
    false,
    "Host Monitoring",
    false,
    concat!(
        rfc_url!(rfc869),
        ",",
        "[Bob Hinden](mailto:bob.hinden&gmail.com)"
    )
);
define_protocol_number!(
    21,
    PRM,
    false,
    "Packet Radio Measurement",
    false,
    "[Zaw-Sing Su](mailto:ZSu&tsca.istc.sri.)"
);
define_protocol_number!(
    22,
    XNS_IDP,
    false,
    "XEROX NS IDP",
    false,
    "[\"The Ethernet, A Local Area Network: Data Link Layer and Physical Layer Specification\", \
    AA-K759B-TK, Digital Equipment Corporation, Maynard, MA. \
    Also as: \"The Ethernet - A Local Area Network\", Version 1.0, Digital Equipment Corporation, \
    Intel Corporation, Xerox Corporation, September 1980. \
    And: \"The Ethernet, A Local Area Network: Data Link Layer and Physical Layer Specifications\", \
    Digital, Intel and Xerox, November 1982. \
    And: XEROX, \"The Ethernet, A Local Area Network: Data Link Layer and Physical Layer Specification\", \
    X3T51/80-50, Xerox Corporation, Stamford, CT., October 1980.][[XEROX]]"
);
define_protocol_number!(
    23,
    TRUNK_1,
    false,
    "Trunk-1",
    false,
    "[Barry Boehm](mailto:boehm&arpa.mil)"
);
define_protocol_number!(
    24,
    TRUNK_2,
    false,
    "Trunk-2",
    false,
    "[Barry Boehm](mailto:boehm&arpa.mil)"
);
define_protocol_number!(
    25,
    LEAF_1,
    false,
    "Leaf-1",
    false,
    "[Barry Boehm](mailto:boehm&arpa.mil)"
);
define_protocol_number!(
    26,
    LEAF_2,
    false,
    "Leaf-2",
    false,
    "[Barry Boehm](mailto:boehm&arpa.mil)"
);
define_protocol_number!(
    27,
    RDP,
    false,
    "Reliable Data Protocol",
    false,
    concat!(
        rfc_url!(rfc908),
        ",",
        "[Bob Hinden](mailto:bob.hinden&gmail.com)"
    )
);
define_protocol_number!(
    28,
    IRTP,
    false,
    "Internet Reliable Transaction",
    false,
    concat!(
        rfc_url!(rfc938),
        ",",
        "[Trudy Miller](mailto:Trudy&acc.com)"
    )
);
define_protocol_number!(
    29,
    ISO_TP4,
    false,
    "ISO Transport Protocol Class 4",
    false,
    concat!(
        rfc_url!(rfc905),
        ",",
        "[Robert Cole](mailto:robert&CS.UCL.AC.UK)"
    )
);
define_protocol_number!(
    30,
    NETBLT,
    false,
    "Bulk Data Transfer Protocol",
    false,
    concat!(
        rfc_url!(rfc969),
        ",",
        "[David Clark](mailto:ddc&lcs.mit.edu)"
    )
);
define_protocol_number!(
    31,
    MFE_NSP,
    false,
    "MFE Network Services Protocol",
    false,
    "[Shuttleworth, B., \"A Documentary of MFENet, a National Computer Network\", \
    UCRL-52317, Lawrence Livermore Labs, Livermore, California, June 1977.]\
    [Barry Howard](mailto:Howard&nmfecc.llnl.gov)"
);
define_protocol_number!(
    32,
    MERIT_INP,
    false,
    "MERIT Internodal Protocol",
    false,
    "[Hans-Werner Braun](mailto:HWB&mcr.umich.edu)"
);
define_protocol_number!(
    33,
    DCCP,
    false,
    "Datagram Congestion Control Protocol",
    false,
    rfc_url!(rfc4340)
);
define_protocol_number!(
    34,
    PC3,
    false,
    "Third Party Connect Protocol",
    false,
    "[Stuart A. Friedberg](mailto:stuart&cs.wisc.edu)"
);
define_protocol_number!(
    35,
    IDPR,
    false,
    "Inter-Domain Policy Routing Protocol",
    false,
    "[Martha Steenstrup](mailto:MSteenst&bbn.com)"
);
define_protocol_number!(
    36,
    XTP,
    false,
    "XTP",
    false,
    "[Greg Chesson](mailto:Greg&sgi.com)"
);
define_protocol_number!(
    37,
    DDP,
    false,
    "Datagram Delivery Protocol",
    false,
    "[Wesley Craig](mailto:Wesley.Craig&terminator.cc.umich.edu)"
);
define_protocol_number!(
    38,
    IDPR_CMTP,
    false,
    "IDPR Control Message Transport Proto",
    false,
    "[Martha Steenstrup](mailto:MSteenst&bbn.com)"
);
define_protocol_number!(
    39,
    TPPP,
    false,
    "TP++ Transport Protocol",
    false,
    "[Dirk Fromhein](mailto:df&watershed.com)"
);
define_protocol_number!(
    40,
    IL,
    false,
    "IL Transport Protocol",
    false,
    "[Dave Presotto](mailto:presotto&plan9.att.com)"
);
define_protocol_number!(
    41,
    IPv6,
    false,
    "IPv6 encapsulation",
    false,
    rfc_url!(rfc2473)
);
define_protocol_number!(
    42,
    SDRP,
    false,
    "Source Demand Routing Protocol",
    false,
    "[Deborah Estrin](mailto:estrin&usc.edu)"
);
define_protocol_number!(
    43,
    IPv6_Route,
    false,
    "Routing Header for IPv6",
    true,
    "[Steve Deering](mailto:deering&parc.xerox.com)"
);
define_protocol_number!(
    44,
    IPv6_Frag,
    false,
    "Fragment Header for IPv6",
    true,
    "[Steve Deering](mailto:deering&parc.xerox.com)"
);
define_protocol_number!(
    45,
    IDRP,
    false,
    "Inter-Domain Routing Protocol",
    false,
    "[Sue Hares](mailto:skh&merit.edu)"
);
define_protocol_number!(
    46,
    RSVP,
    false,
    "Reservation Protocol",
    false,
    concat!(
        rfc_url!(rfc2205, rfc3209),
        ",",
        "[Bob Braden](mailto:braden&isi.edu)"
    )
);
define_protocol_number!(
    47,
    GRE,
    false,
    "Generic Routing Encapsulation",
    false,
    concat!(rfc_url!(rfc2784), ",", "[Tony Li](mailto:tony.li&tony.li)")
);
define_protocol_number!(
    48,
    DSR,
    false,
    "Dynamic Source Routing Protocol",
    false,
    rfc_url!(rfc4728)
);
define_protocol_number!(49, BNA, false, "BNA", false, "[Gary Salamon]");
define_protocol_number!(
    50,
    ESP,
    false,
    "Encap Security Payload",
    true,
    rfc_url!(rfc4303)
);
define_protocol_number!(
    51,
    AH,
    false,
    "Authentication Header",
    true,
    rfc_url!(rfc4302)
);
define_protocol_number!(
    52,
    I_NLSP,
    false,
    "Integrated Net Layer Security TUBA",
    false,
    "[K. Robert Glenn](mailto:glenn&osi.ncsl.nist.gov)"
);
define_protocol_number!(
    53,
    SWIPE,
    true,
    "IP with Encryption",
    false,
    "[John Ioannidis](mailto:ji&tla.org)"
);
define_protocol_number!(
    54,
    NARP,
    false,
    "NBMA Address Resolution Protocol",
    false,
    rfc_url!(rfc1735)
);
define_protocol_number!(
    55,
    Min_IPv4,
    false,
    "Minimal IPv4 Encapsulation",
    false,
    concat!(
        rfc_url!(rfc2004),
        ",",
        "[Charlie Perkins](mailto:perk&watson.ibm.com)"
    )
);
define_protocol_number!(
    56,
    TLSP,
    false,
    "Transport Layer Security Protocol using Kryptonet key management",
    false,
    "[Christer Oberg](mailto:chg&bull.se)"
);
define_protocol_number!(
    57,
    SKIP,
    false,
    "SKIP",
    false,
    "[Tom Markson](mailto:markson&osmosys.ingog.com)"
);
define_protocol_number!(
    58,
    IPv6_ICMP,
    false,
    "ICMP for IPv6",
    false,
    rfc_url!(rfc8200)
);
define_protocol_number!(
    59,
    IPv6_NoNxt,
    false,
    "No Next Header for IPv6",
    false,
    rfc_url!(rfc8200)
);
define_protocol_number!(
    60,
    IPv6_Opts,
    false,
    "Destination Options for IPv6",
    true,
    rfc_url!(rfc8200)
);
define_protocol_number!(
    61,
    HOST_INTERNAL,
    false,
    "any host internal protocol",
    false,
    "[Internet Assigned Numbers Authority](mailto:iana&iana.org)"
);
define_protocol_number!(
    62,
    CFTP,
    false,
    "CFTP",
    false,
    "[Forsdick, H., \"CFTP\", Network Message, Bolt Beranek and Newman, January 1982.]\
    [Harry Forsdick](mailto:Forsdick&bbn.com)"
);
define_protocol_number!(
    63,
    LOCAL_NETWORK,
    false,
    "any local network",
    false,
    "[Internet Assigned Numbers Authority](mailto:iana&iana.org)"
);
define_protocol_number!(
    64,
    SAT_EXPAK,
    false,
    "SATNET and Backroom EXPAK",
    false,
    "[Steven Blumenthal](mailto:BLUMENTHAL&vax.bbn.com)"
);
define_protocol_number!(65, KRYPTOLAN, false, "Kryptolan", false, "[Paul Liu]");
define_protocol_number!(
    66,
    RVD,
    false,
    "MIT Remote Virtual Disk Protocol",
    false,
    "[Michael Greenwald](mailto:Greenwald&scrc-stony-brook.symbolics.com)"
);
define_protocol_number!(
    67,
    IPPC,
    false,
    "Internet Pluribus Packet Core",
    false,
    "[Steven Blumenthal](mailto:BLUMENTHAL&vax.bbn.com)"
);
define_protocol_number!(
    68,
    DISTRIBUTED_FILE_SYSTEM,
    false,
    "any distributed file system",
    false,
    "[Internet Assigned Numbers Authority](mailto:iana&iana.org)"
);
define_protocol_number!(
    69,
    SAT_MON,
    false,
    "SATNET Monitoring",
    false,
    "[Steven Blumenthal](mailto:BLUMENTHAL&vax.bbn.com)"
);
define_protocol_number!(
    70,
    VISA,
    false,
    "VISA Protocol",
    false,
    "[Gene Tsudik](mailto:tsudik&usc.edu)"
);
define_protocol_number!(
    71,
    IPCV,
    false,
    "Internet Packet Core Utility",
    false,
    "[Steven Blumenthal](mailto:BLUMENTHAL&vax.bbn.com)"
);
define_protocol_number!(
    72,
    CPNX,
    false,
    "Computer Protocol Network Executive",
    false,
    "[David Mittnacht]"
);
define_protocol_number!(
    73,
    CPHB,
    false,
    "Computer Protocol Heart Beat",
    false,
    "[David Mittnacht]"
);
define_protocol_number!(
    74,
    WSN,
    false,
    "Wang Span Network",
    false,
    "[Victor Dafoulas]"
);
define_protocol_number!(
    75,
    PVP,
    false,
    "Packet Video Protocol",
    false,
    "[Steve Casner](mailto:casner&isi.edu)"
);
define_protocol_number!(
    76,
    BR_SAT_MON,
    false,
    "Backroom SATNET Monitoring",
    false,
    "[Steven Blumenthal](mailto:BLUMENTHAL&vax.bbn.com)"
);
define_protocol_number!(
    77,
    SUN_ND,
    false,
    "SUN ND PROTOCOL-Temporary",
    false,
    "[William Melohn](mailto:Melohn&sun.com)"
);
define_protocol_number!(
    78,
    WB_MON,
    false,
    "WIDEBAND Monitoring",
    false,
    "[Steven Blumenthal](mailto:BLUMENTHAL&vax.bbn.com)"
);
define_protocol_number!(
    79,
    WB_EXPAK,
    false,
    "WIDEBAND EXPAK",
    false,
    "[Steven Blumenthal](mailto:BLUMENTHAL&vax.bbn.com)"
);
define_protocol_number!(
    80,
    ISO_IP,
    false,
    "ISO Internet Protocol",
    false,
    "[Marshall T. Rose](mailto:mrose&dbc.mtview.ca.us)"
);
define_protocol_number!(
    81,
    VMTP,
    false,
    "VMTP",
    false,
    "[Dave Cheriton](mailto:cheriton&pescadero.stanford.edu)"
);
define_protocol_number!(
    82,
    SECURE_VMTP,
    false,
    "SECURE-VMTP",
    false,
    "[Dave Cheriton](mailto:cheriton&pescadero.stanford.edu)"
);
define_protocol_number!(83, VINES, false, "VINES", false, "[Brian Horn]");
define_protocol_number!(
    84,
    IPTM,
    false,
    "Internet Protocol Traffic Manager",
    false,
    "[Jim Stevens](mailto:jasteven&rockwellcollins.com),\
    note: Until March 2023, value 84 was also assigned to TTP (Transaction Transport Protocol)."
);
define_protocol_number!(
    85,
    NSFNET_IGP,
    false,
    "NSFNET-IGP",
    false,
    "[Hans-Werner Braun](mailto:HWB&mcr.umich.edu)"
);
define_protocol_number!(
    86,
    DGP,
    false,
    "Dissimilar Gateway Protocol",
    false,
    "[M/A-COM Government Systems, \"Dissimilar Gateway Protocol Specification, Draft Version\", \
    Contract no. CS901145, November 16, 1987.][Mike_Little]"
);
define_protocol_number!(
    87,
    TCF,
    false,
    "TCF",
    false,
    "[Guillermo A. Loyola](mailto:LOYOLA&ibm.com)"
);
define_protocol_number!(88, EIGRP, false, "EIGRP", false, rfc_url!(rfc7868));
define_protocol_number!(
    89,
    OSPFIGP,
    false,
    "OSPFIGP",
    false,
    concat!(
        rfc_url!(rfc1583, rfc2328, rfc5340),
        ",",
        "[John Moy](mailto:jmoy&proteon.com)"
    )
);
define_protocol_number!(
    90,
    Sprite_RPC,
    false,
    "Sprite RPC Protocol",
    false,
    "[Welch, B., \"The Sprite Remote Procedure Call System\", \
    Technical Report, UCB/Computer Science Dept., 86/302, University of California at Berkeley, June 1986.][Bruce Willins]"
);
define_protocol_number!(
    91,
    LARP,
    false,
    "Locus Address Resolution Protocol",
    false,
    "[Brian Horn]"
);
define_protocol_number!(
    92,
    MTP,
    false,
    "Multicast Transport Protocol",
    false,
    "[Susie Armstrong](mailto:Armstrong.wbst128&xerox.com)"
);
define_protocol_number!(
    93,
    AX_25,
    false,
    "AX.25 Frames",
    false,
    "[Brian Kantor](mailto:brian&ucsd.edu)"
);
define_protocol_number!(
    94,
    IPIP,
    false,
    "IP-within-IP Encapsulation Protocol",
    false,
    "[John Ioannidis](mailto:ji&tla.org)"
);
define_protocol_number!(
    95,
    MICP,
    true,
    "Mobile Internetworking Control Pro.",
    false,
    "[John Ioannidis](mailto:ji&tla.org)"
);
define_protocol_number!(
    96,
    SCC_SP,
    false,
    "Semaphore Communications Sec. Pro.",
    false,
    "[Howard Hart](mailto:hch&hybrid.com)"
);
define_protocol_number!(
    97,
    ETHERIP,
    false,
    "Ethernet-within-IP Encapsulation",
    false,
    rfc_url!(rfc3378)
);
define_protocol_number!(
    98,
    ENCAP,
    false,
    "Encapsulation Header",
    false,
    concat!(
        rfc_url!(rfc1241),
        ",",
        "[Robert Woodburn](mailto:woody&cseic.saic.com)"
    )
);
define_protocol_number!(
    99,
    PRIVATE_ENCRYPTION_SCHEME,
    false,
    "any private encryption scheme",
    false,
    "[Internet Assigned Numbers Authority](mailto:iana&iana.org)"
);
define_protocol_number!(100, GMTP, false, "GMTP", false, "[[RXB5]]");
define_protocol_number!(
    101,
    IFMP,
    false,
    "Ipsilon Flow Management Protocol",
    false,
    "[Bob Hinden](mailto:bob.hinden&gmail.com),[November 1995, 1997.]"
);
define_protocol_number!(
    102,
    PNNI,
    false,
    "PNNI over IP",
    false,
    "[Ross Callon](mailto:rcallon&baynetworks.com)"
);
define_protocol_number!(
    103,
    PIM,
    false,
    "Protocol Independent Multicast",
    false,
    concat!(
        rfc_url!(rfc7761),
        ",",
        "[Dino Farinacci](mailto:dino&cisco.com)"
    )
);
define_protocol_number!(
    104,
    ARIS,
    false,
    "ARIS",
    false,
    "[Nancy Feldman](mailto:nkf&vnet.ibm.com)"
);
define_protocol_number!(
    105,
    SCPS,
    false,
    "SCPS",
    false,
    "[Robert Durst](mailto:durst&mitre.org)"
);
define_protocol_number!(
    106,
    QNX,
    false,
    "QNX",
    false,
    "[Michael Hunter](mailto:mphunter&qnx.com)"
);
define_protocol_number!(
    107,
    A_N,
    false,
    "Active Networks",
    false,
    "[Bob Braden](mailto:braden&isi.edu)"
);
define_protocol_number!(
    108,
    IPComp,
    false,
    "IP Payload Compression Protocol",
    false,
    rfc_url!(rfc2393)
);
define_protocol_number!(
    109,
    SNP,
    false,
    "Sitara Networks Protocol",
    false,
    "[Manickam R. Sridhar](mailto:msridhar&sitaranetworks.com)"
);
define_protocol_number!(
    110,
    Compaq_Peer,
    false,
    "Compaq Peer Protocol",
    false,
    "[Victor Volpe](mailto:vvolpe&smtp.microcom.com)"
);
define_protocol_number!(
    111,
    IPX_in_IP,
    false,
    "IPX in IP",
    false,
    "[CJ Lee](mailto:cj_lee&novell.com)"
);
define_protocol_number!(
    112,
    VRRP,
    false,
    "Virtual Router Redundancy Protocol",
    false,
    rfc_url!(rfc9568)
);
define_protocol_number!(
    113,
    PGM,
    false,
    "PGM Reliable Transport Protocol",
    false,
    "[Tony Speakman](mailto:speakman&cisco.com)"
);
define_protocol_number!(
    114,
    ZERO_HOP_PROTOCOL,
    false,
    "any 0-hop protocol",
    false,
    "[Internet Assigned Numbers Authority](mailto:iana&iana.org)"
);
define_protocol_number!(
    115,
    L2TP,
    false,
    "Layer Two Tunneling Protocol",
    false,
    concat!(
        rfc_url!(rfc3931),
        ",",
        "[Bernard Aboba](mailto:bernarda&microsoft.com)"
    )
);
define_protocol_number!(
    116,
    DDX,
    false,
    "D-II Data Exchange (DDX)",
    false,
    "[John Worley](mailto:worley&milehigh.net)"
);
define_protocol_number!(
    117,
    IATP,
    false,
    "Interactive Agent Transfer Protocol",
    false,
    "[John Murphy](mailto:john.m.murphy&mci.com)"
);
define_protocol_number!(
    118,
    STP,
    false,
    "Schedule Transfer Protocol",
    false,
    "[Jean-Michel Pittet](mailto:jmp&gandalf.engr.sgi.com)"
);
define_protocol_number!(
    119,
    SRP,
    false,
    "SpectraLink Radio Protocol",
    false,
    "[Mark Hamilton](mailto:mah&spectralink.com)"
);
define_protocol_number!(
    120,
    UTI,
    false,
    "UTI",
    false,
    "[Peter Lothberg](mailto:roll&stupi.se)"
);
define_protocol_number!(
    121,
    SMP,
    false,
    "Simple Message Protocol",
    false,
    "[Leif Ekblad](mailto:leif&rdos.net)"
);
define_protocol_number!(
    122,
    SM,
    true,
    "Simple Multicast Protocol",
    false,
    "[Jon Crowcroft](mailto:jon&cs.ucl.ac.uk),\
[draft-perlman-simple-multicast-03](https://datatracker.ietf.org/doc/draft-perlman-simple-multicast/03/)"
);
define_protocol_number!(
    123,
    PTP,
    false,
    "Performance Transparency Protocol",
    false,
    "[Michael Welzl](mailto:michael&tk.uni-linz.ac.at)"
);
define_protocol_number!(
    124,
    ISIS_over_IPv4,
    false,
    "",
    false,
    "[Tony Przygienda](mailto:prz&siara.com)"
);
define_protocol_number!(
    125,
    FIRE,
    false,
    "",
    false,
    "[Criag Partridge](mailto:craig&bbn.com)"
);
define_protocol_number!(
    126,
    CRTP,
    false,
    "Combat Radio Transport Protocol",
    false,
    "[Robert Sautter](mailto:rsautter&acdnj.itt.com)"
);
define_protocol_number!(
    127,
    CRUDP,
    false,
    "Combat Radio User Datagram",
    false,
    "[Robert Sautter](mailto:rsautter&acdnj.itt.com)"
);
define_protocol_number!(
    128,
    SSCOPMCE,
    false,
    "",
    false,
    "[Kurt Waber](mailto:kurt.waber&swisscom.com)"
);
define_protocol_number!(129, IPLT, false, "", false, "[[Hollbach]]");
define_protocol_number!(
    130,
    SPS,
    false,
    "Secure Packet Shield",
    false,
    "[Bill McIntosh](mailto:BMcIntosh&fortresstech.com)"
);
define_protocol_number!(
    131,
    PIPE,
    false,
    "Private IP Encapsulation within IP",
    false,
    "[Bernhard Petri](mailto:bernhard.petri&siemens.com)"
);
define_protocol_number!(
    132,
    SCTP,
    false,
    "Stream Control Transmission Protocol",
    false,
    "[Randall R. Stewart](mailto:rrs&lakerest.net)"
);
define_protocol_number!(
    133,
    FC,
    false,
    "Fibre Channel",
    false,
    concat!(
        "[Murali Rajagopal](mailto:murali&gadzoox.com)",
        ",",
        rfc_url!(rfc6172)
    )
);
define_protocol_number!(134, RSVP_E2E_IGNORE, false, "", false, rfc_url!(rfc3175));
define_protocol_number!(135, Mobility_Header, false, "", true, rfc_url!(rfc6275));
define_protocol_number!(136, UDPLite, false, "", false, rfc_url!(rfc3828));
define_protocol_number!(137, MPLS_in_IP, false, "", false, rfc_url!(rfc4023));
define_protocol_number!(
    138,
    manet,
    false,
    "MANET Protocols",
    false,
    rfc_url!(rfc5498)
);
define_protocol_number!(
    139,
    HIP,
    false,
    "Host Identity Protocol",
    true,
    rfc_url!(rfc7401)
);
define_protocol_number!(140, Shim6, false, "Shim6 Protocol", true, rfc_url!(rfc5533));
define_protocol_number!(
    141,
    WESP,
    false,
    "Wrapped Encapsulating Security Payload",
    false,
    rfc_url!(rfc5840)
);
define_protocol_number!(
    142,
    ROHC,
    false,
    "Robust Header Compression",
    false,
    rfc_url!(rfc5858)
);
define_protocol_number!(143, Ethernet, false, "Ethernet", false, rfc_url!(rfc8986));
define_protocol_number!(
    144,
    AGGFRAG,
    false,
    "AGGFRAG encapsulation payload for ESP",
    false,
    rfc_url!(rfc9347)
);
define_protocol_number!(
    145,
    NSH,
    false,
    "Network Service Header",
    false,
    rfc_url!(rfc9491)
);
define_protocol_number!(
    146,
    Homa,
    false,
    "Homa",
    false,
    "[HomaModule],\
[John Ousterhout](mailto:john.ousterhout&gmail.com)"
);
define_protocol_number!(
    147,
    BIT_EMU,
    false,
    "Bit-stream Emulation",
    true,
    rfc_url!(rfc9801)
);

#[cfg(test)]
mod test {
    use crate::protocol_numbers::*;
    #[test]
    fn print_all() {
        // Print all protocol numbers and their keywords
        let protocols: Vec<Box<dyn ProtocolNumberDynamic>> = vec![
            Box::new(HOPOPT),
            Box::new(ICMP),
            Box::new(IGMP),
            Box::new(GGP),
            Box::new(IPv4),
            Box::new(ST),
            Box::new(TCP),
            Box::new(CBT),
            Box::new(EGP),
            Box::new(IGP),
            Box::new(BBN_RCC_MON),
            Box::new(NVP_II),
            Box::new(PUP),
            Box::new(ARGUS),
            Box::new(EMCON),
            Box::new(XNET),
            Box::new(CHAOS),
            Box::new(UDP),
            Box::new(MUX),
            Box::new(DCN_MEAS),
            Box::new(HMP),
            Box::new(PRM),
            Box::new(XNS_IDP),
            Box::new(TRUNK_1),
            Box::new(TRUNK_2),
            Box::new(LEAF_1),
            Box::new(LEAF_2),
            Box::new(RDP),
            Box::new(IRTP),
            Box::new(ISO_TP4),
            Box::new(NETBLT),
            Box::new(MFE_NSP),
            Box::new(MERIT_INP),
            Box::new(DCCP),
            Box::new(PC3),
            Box::new(IDPR),
            Box::new(XTP),
            Box::new(DDP),
            Box::new(IDPR_CMTP),
            Box::new(TPPP),
            Box::new(IL),
            Box::new(IPv6),
            Box::new(SDRP),
            Box::new(IPv6_Route),
            Box::new(IPv6_Frag),
            Box::new(IDRP),
            Box::new(RSVP),
            Box::new(GRE),
            Box::new(DSR),
            Box::new(BNA),
            Box::new(ESP),
            Box::new(AH),
            Box::new(I_NLSP),
            Box::new(SWIPE),
            Box::new(NARP),
            Box::new(Min_IPv4),
            Box::new(TLSP),
            Box::new(SKIP),
            Box::new(IPv6_ICMP),
            Box::new(IPv6_NoNxt),
            Box::new(IPv6_Opts),
            Box::new(HOST_INTERNAL),
            Box::new(CFTP),
            Box::new(LOCAL_NETWORK),
            Box::new(SAT_EXPAK),
            Box::new(KRYPTOLAN),
            Box::new(RVD),
            Box::new(IPPC),
            Box::new(DISTRIBUTED_FILE_SYSTEM),
            Box::new(SAT_MON),
            Box::new(VISA),
            Box::new(IPCV),
            Box::new(CPNX),
            Box::new(CPHB),
            Box::new(WSN),
            Box::new(PVP),
            Box::new(BR_SAT_MON),
            Box::new(SUN_ND),
            Box::new(WB_MON),
            Box::new(WB_EXPAK),
            Box::new(ISO_IP),
            Box::new(VMTP),
            Box::new(SECURE_VMTP),
            Box::new(VINES),
            Box::new(IPTM),
            Box::new(NSFNET_IGP),
            Box::new(DGP),
            Box::new(TCF),
            Box::new(EIGRP),
            Box::new(OSPFIGP),
            Box::new(Sprite_RPC),
            Box::new(LARP),
            Box::new(MTP),
            Box::new(AX_25),
            Box::new(IPIP),
            Box::new(MICP),
            Box::new(SCC_SP),
            Box::new(ETHERIP),
            Box::new(ENCAP),
            Box::new(PRIVATE_ENCRYPTION_SCHEME),
            Box::new(GMTP),
            Box::new(IFMP),
            Box::new(PNNI),
            Box::new(PIM),
            Box::new(ARIS),
            Box::new(SCPS),
            Box::new(QNX),
            Box::new(A_N),
            Box::new(IPComp),
            Box::new(SNP),
            Box::new(Compaq_Peer),
            Box::new(IPX_in_IP),
            Box::new(VRRP),
            Box::new(PGM),
            Box::new(ZERO_HOP_PROTOCOL),
            Box::new(L2TP),
            Box::new(DDX),
            Box::new(IATP),
            Box::new(STP),
            Box::new(SRP),
            Box::new(UTI),
            Box::new(SMP),
            Box::new(SM),
            Box::new(PTP),
            Box::new(ISIS_over_IPv4),
            Box::new(FIRE),
            Box::new(CRTP),
            Box::new(CRUDP),
            Box::new(SSCOPMCE),
            Box::new(IPLT),
            Box::new(SPS),
            Box::new(PIPE),
            Box::new(SCTP),
            Box::new(FC),
            Box::new(RSVP_E2E_IGNORE),
            Box::new(Mobility_Header),
            Box::new(UDPLite),
            Box::new(MPLS_in_IP),
            Box::new(manet),
            Box::new(HIP),
            Box::new(Shim6),
            Box::new(WESP),
            Box::new(ROHC),
            Box::new(Ethernet),
            Box::new(AGGFRAG),
            Box::new(NSH),
            Box::new(Homa),
            Box::new(BIT_EMU),
        ];
        assert_eq!(protocols.len(), 148);
        for proto in protocols {
            println!(
                "{}: {} (Deprecated: {}), {}, {}",
                proto.decimal(),
                proto.keyword(),
                proto.deprecated(),
                proto.protocol_name(),
                proto.reference(),
            );
        }
    }

    #[test]
    fn test_protocol_number_static_trait() {
        assert_eq!(TCP::DECIMAL, 6);
        assert_eq!(TCP::KEYWORD, "TCP");
        assert_eq!(TCP::PROTOCOL_NAME, "Transmission Control");
        assert_eq!(TCP::IPV6_EXTENSION_HEADER, false);
        assert!(TCP::REFERENCE.contains("rfc9293"));
        assert_eq!(TCP::DEPRECATED, false);
    }

    #[test]
    fn test_protocol_number_dynamic_trait() {
        let udp = UDP;
        assert_eq!(udp.decimal(), 17);
        assert_eq!(udp.keyword(), "UDP");
        assert_eq!(udp.protocol_name(), "User Datagram");
        assert_eq!(udp.ipv6_extension_header(), false);
        assert!(udp.reference().contains("rfc768"));
        assert_eq!(udp.deprecated(), false);
    }

    #[test]
    fn test_deprecated_protocols() {
        let argus = ARGUS;
        let swipe = SWIPE;
        let micp = MICP;
        let sm = SM;
        assert_eq!(argus.deprecated(), true);
        assert_eq!(swipe.deprecated(), true);
        assert_eq!(micp.deprecated(), true);
        assert_eq!(sm.deprecated(), true);
    }

    #[test]
    fn test_ipv6_extension_headers() {
        assert_eq!(HOPOPT::IPV6_EXTENSION_HEADER, true);
        assert_eq!(IPv6_Route::IPV6_EXTENSION_HEADER, true);
        assert_eq!(IPv6_Frag::IPV6_EXTENSION_HEADER, true);
        assert_eq!(ESP::IPV6_EXTENSION_HEADER, true);
        assert_eq!(AH::IPV6_EXTENSION_HEADER, true);
        assert_eq!(IPv6_Opts::IPV6_EXTENSION_HEADER, true);
        assert_eq!(Mobility_Header::IPV6_EXTENSION_HEADER, true);
        assert_eq!(HIP::IPV6_EXTENSION_HEADER, true);
        assert_eq!(Shim6::IPV6_EXTENSION_HEADER, true);
        assert_eq!(BIT_EMU::IPV6_EXTENSION_HEADER, true);
        assert_eq!(TCP::IPV6_EXTENSION_HEADER, false);
        assert_eq!(UDP::IPV6_EXTENSION_HEADER, false);
    }
}
