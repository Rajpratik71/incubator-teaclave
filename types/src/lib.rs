#![cfg_attr(feature = "mesalock_sgx", no_std)]
#[cfg(feature = "mesalock_sgx")]
extern crate sgx_tstd as std;

#[cfg(feature = "mesalock_sgx")]
use std::prelude::v1::*;

use hex;
use serde::Deserializer;
use serde_derive::Deserialize;

/// Status for Ecall
#[repr(C)]
#[derive(Debug)]
pub struct EnclaveStatus(pub u32);

/// Status for Ocall
pub type UntrustedStatus = EnclaveStatus;

impl EnclaveStatus {
    pub fn default() -> EnclaveStatus {
        EnclaveStatus(0)
    }

    pub fn is_err(&self) -> bool {
        match self.0 {
            0 => false,
            _ => true,
        }
    }

    pub fn is_err_ffi_outbuf(&self) -> bool {
        self.0 == 0x0000_000c
    }
}

pub type SgxMeasurement = [u8; sgx_types::SGX_HASH_SIZE];

#[derive(Debug, Deserialize, Copy, Clone, Eq, PartialEq)]
pub struct EnclaveMeasurement {
    #[serde(deserialize_with = "from_hex")]
    pub mr_signer: SgxMeasurement,
    #[serde(deserialize_with = "from_hex")]
    pub mr_enclave: SgxMeasurement,
}

impl EnclaveMeasurement {
    pub fn new(mr_enclave: SgxMeasurement, mr_signer: SgxMeasurement) -> Self {
        Self {
            mr_enclave,
            mr_signer,
        }
    }
}

/// Deserializes a hex string to a `SgxMeasurement` (i.e., [0; 32]).
pub fn from_hex<'de, D>(deserializer: D) -> Result<SgxMeasurement, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;
    use serde::Deserialize;
    String::deserialize(deserializer).and_then(|string| {
        let v = hex::decode(&string).map_err(|_| Error::custom("ParseError"))?;
        let mut array = [0; sgx_types::SGX_HASH_SIZE];
        let bytes = &v[..array.len()]; // panics if not enough data
        array.copy_from_slice(bytes);
        Ok(array)
    })
}
