pub mod reader;
pub mod writer;
///
/// In encrypted FLV files, the AdditionalHeader object shall be present,
/// and shall include the Encryption Header object.
/// The AdditionalHeader object shall be carried in a SCRIPTDATA tag named |AdditionalHeader.
/// (Note the vertical bar ('|') in the name.)
/// The object should be present at the beginning of the FLV,
/// with timestamp 0, immediately after the onMetaData ScriptData tag.
/// This gives the FLV decoder access to the encryption metadata before it encounters any encrypted tags.
///
/// In script tag:
/// "|AdditionalHeader": {
///   "Encryption": {
///     "Version": number,
///     "Method": string,
///     "Flags": number,
///     "Params": {
///       "Version": number,
///       "EncryptionAlgorithm": string,
///       "EncryptionParams": {
///         "KeyLength": number
///       }
///       "KeyInfo": {
///         "SubType": "FlashAccessv2",
///         "Data": {
///           Metadata: string
///         }
///       }
///     }
///     (if Version == 1)"SigFormat": string,
///     (if Version == 1)"Signature": string,
///   }
/// }
#[derive(Debug)]
pub struct EncryptionHeader {
    /// Version of Encryption Header.
    /// Shall be 1 or 2, indicating the version of the encryption format.
    /// 1 = FMRMS v1.x products.
    /// 2 = Flash Access 2.0 products.
    /// Contents protected using either version are in existence,
    /// so applications shall be able to consume both versions of the content.
    version: f64,
    /// Encryption method. Shall be ‘Standard’
    method: String,
    /// Encryption flags. Shall be 0.
    flags: f64,
    /// Parameters for encryption method 'Standard'
    params: StandardEncodingParametersObject,
    // IF version == 1
    signature_params: Option<SignatureParams>,
}

#[derive(Debug)]
pub struct SignatureParams {
    sig_format: Option<String>,
    signature: Option<String>,
}

#[derive(Debug)]
pub struct StandardEncodingParametersObject {
    /// Version. Shall be 1.
    version: f64,
    /// The encryption algorithm.
    /// Shall be ‘AES-CBC’, which specifies that the encryption used is ‘AESCBC’ with padding as per RFC 2630.
    encryption_algorithm: String,
    encryption_params: AESCBSEncryptionParamsObject,
    key_info: KeyInformationObject,
}

/// This structure contains parameters specific to the encryption algorithm, in this case AES-CBC_128.
#[derive(Debug)]
pub struct AESCBSEncryptionParamsObject {
    /// Key length for the encryption algorithm in bytes. Shall be 16 (i.e. 128 bits)
    key_length: f64,
}

#[derive(Debug)]
pub struct KeyInformationObject {
    /// IF EncryptionHeader.Version == 1
    ///   ‘APS’ = (Adobe Policy Server) An online key agreement negotiation protocol
    /// ELSE
    ///   ‘FlashAccessv2’ = An online key retrieval protocol
    /// APS should NOT be used
    sub_type: String,
    data: FlashAccessV2Object,
}

#[derive(Debug)]
pub struct FlashAccessV2Object {
    /// Base 64 encoded metadata used by the DRM client to retrieve the decryption key.
    meta_data: String,
}

#[derive(Debug)]
pub struct EncryptionTagHeader {
    /// Number of filters applied to the packet. Shall be 1.
    num_filters: u8,
    /// Name of the filter.
    /// IF EncryptionHeader.Version == 1
    ///   ‘Encryption’
    /// ELSE
    ///   ‘SE’
    /// SE stands for Selective Encryption.
    pub filter_name: String,
    /// Length of FilterParams in bytes
    pub length: u32,
}

use std::io;

use tokio_util::either::Either;

use crate::errors::{FLVError, FLVResult};

#[derive(Debug)]
pub struct FilterParams {
    /// Parameters specific to the filter.
    /// IF FilterName = ‘Encryption’
    ///   EncryptionFilterParams
    /// IF FilterName = ‘SE’
    ///   SelectiveEncryptionFilterParams
    /// "FilterParams"
    pub filter_params: Either<EncryptionFilterParams, SelectiveEncryptionFilterParams>,
}

#[derive(Debug)]
pub struct EncryptionFilterParams {
    /// Contains 16 bytes of IV data for AES-CBC.
    /// "IV"
    iv: [u8; 16],
}

#[derive(Debug)]
pub struct SelectiveEncryptionFilterParams {
    /// Selective Encryption indicator shows if the packet is encrypted.
    /// 0 = packet is not encrypted
    /// 1 = packet is encrypted.
    /// "EncryptedAU"
    // encrypted_au: bool,
    /// F EncryptedAU == 1
    /// Only present if the packet is encrypted.
    /// Contains 16 bytes of IV data for AES-CBC
    iv: Option<[u8; 16]>,
}
