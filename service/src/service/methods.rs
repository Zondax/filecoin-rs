//////! Fcservice RPC Client

use crate::service::error::ServiceError;
use filecoin_signer::api::UnsignedMessageUserAPI;
use filecoin_signer::utils::{from_hex_string, to_hex_string};
use jsonrpc_core::{Id, MethodCall, Success, Version};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub async fn key_generate_mnemonic(_c: MethodCall) -> Result<Success, ServiceError> {
    let mnemonic = filecoin_signer::key_generate_mnemonic()?;

    let so = Success {
        jsonrpc: Some(Version::V2),
        result: Value::from(mnemonic),
        id: Id::Num(1),
    };

    Ok(so)
}

#[derive(Debug, Deserialize, Serialize)]
pub struct KeyDeriveParamsAPI {
    pub mnemonic: String,
    pub path: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct KeyDeriveResultApi {
    pub prvkey: String,
    pub pubkey: String,
    pub address: String,
}

pub async fn key_derive(c: MethodCall) -> Result<Success, ServiceError> {
    let y = c.params.parse::<KeyDeriveParamsAPI>()?;

    let (prvkey, pubkey, address) = filecoin_signer::key_derive(y.mnemonic, y.path)?;

    let result = KeyDeriveResultApi {
        prvkey,
        pubkey,
        address,
    };

    let result_json = serde_json::to_value(&result)?;

    let so = Success {
        jsonrpc: Some(Version::V2),
        result: result_json,
        id: Id::Num(1),
    };

    Ok(so)
}

pub async fn transaction_create(c: MethodCall) -> Result<Success, ServiceError> {
    let y = c.params.parse::<UnsignedMessageUserAPI>()?;
    let cbor_hexstring = filecoin_signer::transaction_create(y)?;

    let so = Success {
        jsonrpc: Some(Version::V2),
        result: Value::from(cbor_hexstring.0),
        id: Id::Num(1),
    };

    Ok(so)
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TransctionParseParamsAPI {
    pub cbor_hex: String,
    pub testnet: bool,
}

pub async fn transaction_parse(c: MethodCall) -> Result<Success, ServiceError> {
    let params = c.params.parse::<TransctionParseParamsAPI>()?;
    let message_parsed =
        filecoin_signer::transaction_parse(params.cbor_hex.as_bytes(), params.testnet)?;
    let tx = serde_json::to_string(&message_parsed)?;

    let so = Success {
        jsonrpc: Some(Version::V2),
        result: Value::from(tx),
        id: Id::Num(1),
    };

    Ok(so)
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SignTransactionParamsAPI {
    pub transaction: UnsignedMessageUserAPI,
    pub prvkey_hex: String,
}

pub async fn sign_transaction(c: MethodCall) -> Result<Success, ServiceError> {
    let params = c.params.parse::<SignTransactionParamsAPI>()?;

    let prvkey_bytes = from_hex_string(&params.prvkey_hex)?;

    let (signed_message, v) = filecoin_signer::sign_transaction(params.transaction, &prvkey_bytes)?;

    let so = Success {
        jsonrpc: Some(Version::V2),
        result: Value::from([to_hex_string(&signed_message), format!("{:02x}", &v)].concat()),
        id: Id::Num(1),
    };

    Ok(so)
}

#[derive(Debug, Deserialize, Serialize)]
pub struct VerifySignatureParamsAPI {
    pub signature_hex: String,
    pub message_hex: String,
}

pub async fn verify_signature(c: MethodCall) -> Result<Success, ServiceError> {
    let params = c.params.parse::<VerifySignatureParamsAPI>()?;

    let signature = from_hex_string(&params.signature_hex)?;

    let result = filecoin_signer::verify_signature(&signature, &params.message_hex.as_bytes())?;

    let so = Success {
        jsonrpc: Some(Version::V2),
        result: Value::from(result),
        id: Id::Num(1),
    };

    Ok(so)
}

#[cfg(test)]
mod tests {
    use crate::service::client::get_nonce;
    use futures_await_test::async_test;

    #[ignore]
    #[async_test]
    async fn example_something_else_and_retrieve_nonce() {
        // FIXME: use configuration parameters instead
        let url = "https://lotus-dev.temporal.cloud/rpc/v0";
        let jwt = "some_token";
        let addr = "t1jdlfl73voaiblrvn2yfivvn5ifucwwv5f26nfza";

        let nonce = get_nonce(&url, &jwt, &addr).await;
        assert!(nonce.is_ok());
    }
}
