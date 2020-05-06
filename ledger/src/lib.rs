/*******************************************************************************
*   (c) 2020 ZondaX GmbH
*
*  Licensed under the Apache License, Version 2.0 (the "License");
*  you may not use this file except in compliance with the License.
*  You may obtain a copy of the License at
*
*      http://www.apache.org/licenses/LICENSE-2.0
*
*  Unless required by applicable law or agreed to in writing, software
*  distributed under the License is distributed on an "AS IS" BASIS,
*  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
*  See the License for the specific language governing permissions and
*  limitations under the License.
********************************************************************************/
//! Support library for Filecoin Ledger Nano S/X apps

#![deny(warnings, trivial_casts, trivial_numeric_casts)]
#![deny(unused_import_braces, unused_qualifications)]
#![deny(missing_docs)]
#![doc(html_root_url = "https://docs.rs/ledger-filecoin/0.1.0")]

extern crate byteorder;
extern crate secp256k1;
use serde::{Deserialize, Serialize};

mod params;

use self::params::{APDUErrors, PayloadType};
use crate::params::*;

pub use ledger_generic::{ApduAnswer, ApduCommand};
use std::str;

mod errors;
use errors::Error;

#[cfg(target_arch = "wasm32")]
mod apdu_transport_wasm;

#[cfg(target_arch = "wasm32")]
pub use crate::apdu_transport_wasm::ApduTransport;

#[cfg(target_arch = "wasm32")]
/// Trait for any APDU transport
pub trait TransportWrapperTrait {
    /// Send an APDU command and receive a promise of a response back
    fn exchange_apdu(&self, apdu_command: &[u8]) -> js_sys::Promise;
}

#[cfg(not(target_arch = "wasm32"))]
mod apdu_transport_native;

#[cfg(not(target_arch = "wasm32"))]
pub use crate::apdu_transport_native::ApduTransport;

/// hex string utilities
pub mod utils;

use crate::utils::{serialize_bip44, BIP44Path};

/// Filecoin App
pub struct FilecoinApp {
    apdu_transport: ApduTransport,
}

unsafe impl Send for FilecoinApp {}

/// FilecoinApp address (includes pubkey and the corresponding ss58 address)
pub struct Address {
    /// Public Key
    pub public_key: secp256k1::PublicKey,

    /// Address byte format
    pub addr_byte: [u8; 21],

    /// Address string format
    pub addr_string: String,
}

/// FilecoinApp signature (includes R, S, V and der format)
pub struct Signature {
    /// r value
    pub r: [u8; 32],

    /// s value
    pub s: [u8; 32],

    /// v value
    pub v: u8,

    /// der signature
    pub sig: secp256k1::Signature,
}

/// FilecoinApp App Version
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Version {
    /// Application Mode
    pub mode: u8,
    /// Version Major
    pub major: u8,
    /// Version Minor
    pub minor: u8,
    /// Version Patch
    pub patch: u8,
}

impl FilecoinApp {
    /// Connect to the Ledger App
    pub fn connect(apdu_transport: ApduTransport) -> Result<Self, Error> {
        Ok(FilecoinApp { apdu_transport })
    }

    /// Retrieve the app version
    pub async fn get_version(&self) -> Result<Version, Error> {
        let command = ApduCommand {
            cla: CLA,
            ins: INS_GET_VERSION,
            p1: 0x00,
            p2: 0x00,
            length: 0,
            data: Vec::new(),
        };

        let response = self.apdu_transport.exchange(command).await?;
        if response.retcode != APDUErrors::NoError as u16 {
            return Err(Error::InvalidVersion);
        }

        if response.data.len() < 4 {
            return Err(Error::InvalidVersion);
        }

        let version = Version {
            mode: response.data[0],
            major: response.data[1],
            minor: response.data[2],
            patch: response.data[3],
        };

        Ok(version)
    }

    /// Retrieves the public key and address
    pub async fn get_address(
        &self,
        path: &BIP44Path,
        require_confirmation: bool,
    ) -> Result<Address, Error> {
        let serialized_path = serialize_bip44(path);
        let p1 = if require_confirmation { 1 } else { 0 };

        let command = ApduCommand {
            cla: CLA,
            ins: INS_GET_ADDR_SECP256K1,
            p1,
            p2: 0x00,
            length: 0,
            data: serialized_path,
        };

        match self.apdu_transport.exchange(command).await {
            Ok(response) => {
                if response.retcode != APDUErrors::NoError as u16 {
                    println!("WARNING: retcode={:X?}", response.retcode);
                }

                if response.data.len() < PK_LEN {
                    return Err(Error::InvalidPK);
                }

                let public_key = secp256k1::PublicKey::parse_slice(
                    &response.data[..PK_LEN],
                    Some(secp256k1::PublicKeyFormat::Full),
                )
                .map_err(|_| errors::Error::Secp256k1)?;

                let mut addr_byte = [Default::default(); 21];
                addr_byte.copy_from_slice(&response.data[PK_LEN + 1..PK_LEN + 1 + 21]);

                let tmp =
                    str::from_utf8(&response.data[PK_LEN + 2 + 21..]).map_err(|_e| Error::Utf8)?;
                let addr_string = tmp.to_owned();

                let address = Address {
                    public_key,
                    addr_byte,
                    addr_string,
                };
                Ok(address)
            }

            // FIXME
            Err(_) => Err(Error::TransportError),
        }
    }

    /// Sign a transaction
    pub async fn sign(&self, path: &BIP44Path, message: &[u8]) -> Result<Signature, Error> {
        let bip44path = serialize_bip44(&path);
        let chunks = message.chunks(USER_MESSAGE_CHUNK_SIZE);

        if chunks.len() > 255 {
            return Err(Error::InvalidMessageSize);
        }

        if chunks.len() == 0 {
            return Err(Error::InvalidEmptyMessage);
        }

        let packet_count = chunks.len() as u8;
        let mut response: ApduAnswer;

        let _command = ApduCommand {
            cla: CLA,
            ins: INS_SIGN_SECP256K1,
            p1: PayloadType::Init as u8,
            p2: 0x00,
            length: bip44path.len() as u8,
            data: bip44path,
        };

        response = self.apdu_transport.exchange(_command).await?;

        // Send message chunks
        for (packet_idx, chunk) in chunks.enumerate() {
            let mut p1 = PayloadType::Add as u8;
            if packet_idx == (packet_count - 1) as usize {
                p1 = PayloadType::Last as u8
            }

            let _command = ApduCommand {
                cla: CLA,
                ins: INS_SIGN_SECP256K1,
                p1,
                p2: 0,
                length: chunk.len() as u8,
                data: chunk.to_vec(),
            };

            response = self.apdu_transport.exchange(_command).await?;
        }

        if response.data.is_empty() && response.retcode == APDUErrors::NoError as u16 {
            return Err(Error::NoSignature);
        }

        // Last response should contain the answer
        if response.data.len() < 3 {
            return Err(Error::InvalidSignature);
        }

        let mut r = [Default::default(); 32];
        r.copy_from_slice(&response.data[..32]);

        let mut s = [Default::default(); 32];
        s.copy_from_slice(&response.data[32..64]);

        let v = response.data[64];

        let sig = secp256k1::Signature::parse_der(&response.data[65..])
            .map_err(|_| errors::Error::Secp256k1)?;

        let signature = Signature { r, s, v, sig };

        Ok(signature)
    }
}

#[cfg(test)]
mod tests {}
