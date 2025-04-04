use url::Url;

use crate::{
    attributes::{SDPAttribute, SDPTrivialAttribute},
    session::{
        SDPAddrType, SDPAddress, SDPBandWidthInformation, SDPConnectionInformation,
        SDPEncryptionKeys, SDPMediaDescription, SDPNetType, SDPRepeatTime, SDPTimeInformation,
        SDPVersion, SessionDescription,
    },
};

#[derive(Debug, Default)]
pub struct SdpBuilder {
    session_description: SessionDescription,
}

impl SdpBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn version(mut self, version: SDPVersion) -> Self {
        self.session_description.version = version;
        self
    }

    pub fn origin_user_name(mut self, user_name: String) -> Self {
        self.session_description.origin.user_name = user_name;
        self
    }

    pub fn origin_session_id(mut self, session_id: u64) -> Self {
        self.session_description.origin.session_id = session_id;
        self
    }

    pub fn origin_session_version(mut self, session_version: u64) -> Self {
        self.session_description.origin.session_version = session_version;
        self
    }

    pub fn origin_net_type(mut self, net_type: SDPNetType) -> Self {
        self.session_description.origin.net_type = net_type;
        self
    }

    pub fn origin_addr_type(mut self, addr_type: SDPAddrType) -> Self {
        self.session_description.origin.addr_type = addr_type;
        self
    }

    pub fn origin_unicast_address(mut self, unicast_address: String) -> Self {
        self.session_description.origin.unicast_address = unicast_address;
        self
    }

    pub fn session_name(mut self, session_name: String) -> Self {
        self.session_description.session_name = session_name;
        self
    }

    pub fn session_info(mut self, session_info: String) -> Self {
        self.session_description.session_information = Some(session_info);
        self
    }

    pub fn uri(mut self, uri: Url) -> Self {
        self.session_description.uri = Some(uri);
        self
    }

    pub fn email(mut self, email: String) -> Self {
        self.session_description.email_address.push(email);
        self
    }

    pub fn phone(mut self, phone: String) -> Self {
        self.session_description.phone_number.push(phone);
        self
    }

    pub fn connection_info(
        mut self,
        net_type: SDPNetType,
        addr_type: SDPAddrType,
        connection_address: String,
        ttl: Option<u64>,
        range: Option<u64>,
    ) -> Self {
        self.session_description.connection_information = Some(SDPConnectionInformation {
            net_type,
            addr_type,
            connection_address: SDPAddress {
                address: connection_address,
                ttl,
                range,
            },
        });
        self
    }

    pub fn bandwidth_info(
        mut self,
        bw_type: crate::session::SDPBandwidthType,
        bandwidth: u64,
    ) -> Self {
        self.session_description
            .bandwidth_information
            .push(SDPBandWidthInformation { bw_type, bandwidth });
        self
    }

    pub fn time_info(
        mut self,
        start_time: u64,
        stop_time: u64,
        repeat_times: Vec<SDPRepeatTime>,
    ) -> Self {
        self.session_description
            .time_information
            .push(SDPTimeInformation {
                start_time,
                stop_time,
                repeat_times,
            });
        self
    }

    pub fn encryption_keys(mut self, method: String, key: Option<String>) -> Self {
        self.session_description.encryption_keys = Some(SDPEncryptionKeys { method, key });
        self
    }

    pub fn attribute(mut self, name: String, value: Option<String>) -> Self {
        self.session_description
            .attributes
            .push(SDPAttribute::Trivial(SDPTrivialAttribute { name, value }));
        self
    }

    pub fn media_description(mut self, media: SDPMediaDescription) -> Self {
        self.session_description.media_description.push(media);
        self
    }

    pub fn build(self) -> SessionDescription {
        self.session_description
    }
}
