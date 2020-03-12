//////! Filecoin Service RPC Client

use crate::config::RemoteNodeSection;
use crate::service::client;
use crate::service::error::ServiceError;
use filecoin_signer::api::UnsignedMessageAPI;
use filecoin_signer::utils::{from_hex_string, to_hex_string};
use filecoin_signer::{CborBuffer, Mnemonic, SecretKey, Signature};
use jsonrpc_core::{Id, MethodCall, Success, Version};
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::Value;
use std::convert::TryFrom;

#[derive(Debug, Deserialize, Serialize)]
pub struct SignTransactionParamsAPI {
    pub transaction: UnsignedMessageAPI,
    pub prvkey_hex: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GetNonceParamsAPI {
    pub account: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct KeyDeriveParamsAPI {
    pub mnemonic: String,
    pub path: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct KeyDeriveResultAPI {
    pub private_hexstring: String,
    pub public_hexstring: String,
    pub address: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TransctionParseParamsAPI {
    pub cbor_hex: String,
    pub testnet: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct VerifySignatureParamsAPI {
    pub signature_hex: String,
    pub message_hex: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GetStatusParamsAPI {
    pub cid_message: String,
}

pub async fn key_generate_mnemonic(
    _c: MethodCall,
    _: RemoteNodeSection,
) -> Result<Success, ServiceError> {
    let mnemonic = filecoin_signer::key_generate_mnemonic()?;

    let so = Success {
        jsonrpc: Some(Version::V2),
        result: Value::from(mnemonic.0),
        id: Id::Num(1),
    };

    Ok(so)
}

pub async fn key_derive(c: MethodCall, _: RemoteNodeSection) -> Result<Success, ServiceError> {
    let params = c.params.parse::<KeyDeriveParamsAPI>()?;

    let (private, public, address) =
        filecoin_signer::key_derive(Mnemonic(params.mnemonic), params.path)?;

    let result = KeyDeriveResultAPI {
        public_hexstring: to_hex_string(&public.0[..]),
        private_hexstring: to_hex_string(&private.0[..]),
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

pub async fn transaction_serialize(
    c: MethodCall,
    _: RemoteNodeSection,
) -> Result<Success, ServiceError> {
    let params = c.params.parse::<UnsignedMessageAPI>()?;
    let cbor_hexstring = filecoin_signer::transaction_serialize(params)?;

    let so = Success {
        jsonrpc: Some(Version::V2),
        result: Value::from(cbor_hexstring.0),
        id: Id::Num(1),
    };

    Ok(so)
}

pub async fn transaction_parse(
    c: MethodCall,
    _: RemoteNodeSection,
) -> Result<Success, ServiceError> {
    let params = c.params.parse::<TransctionParseParamsAPI>()?;
    let cbor_data = CborBuffer(from_hex_string(params.cbor_hex.as_ref()).unwrap());

    let message_parsed = filecoin_signer::transaction_parse(&cbor_data, params.testnet)?;

    let tx = serde_json::to_string(&message_parsed)?;

    let so = Success {
        jsonrpc: Some(Version::V2),
        result: Value::from(tx),
        id: Id::Num(1),
    };

    Ok(so)
}

pub async fn sign_transaction(
    c: MethodCall,
    _: RemoteNodeSection,
) -> Result<Success, ServiceError> {
    let params = c.params.parse::<SignTransactionParamsAPI>()?;

    let private_key = SecretKey::try_from(params.prvkey_hex)?;

    let signature = filecoin_signer::sign_transaction(params.transaction, &private_key)?;

    let so = Success {
        jsonrpc: Some(Version::V2),
        result: Value::from(to_hex_string(&signature.0)),
        id: Id::Num(1),
    };

    Ok(so)
}

pub async fn verify_signature(
    c: MethodCall,
    _: RemoteNodeSection,
) -> Result<Success, ServiceError> {
    let params = c.params.parse::<VerifySignatureParamsAPI>()?;

    let signature = Signature::try_from(params.signature_hex)?;
    let message = CborBuffer(from_hex_string(params.message_hex.as_ref()).unwrap());

    let result = filecoin_signer::verify_signature(&signature, &message)?;

    let so = Success {
        jsonrpc: Some(Version::V2),
        result: Value::from(result),
        id: Id::Num(1),
    };

    Ok(so)
}

pub async fn get_status(c: MethodCall, config: RemoteNodeSection) -> Result<Success, ServiceError> {
    let call_params = c.params.parse::<GetStatusParamsAPI>()?;
    let params = json!({"/": call_params.cid_message.to_string()});

    let result = client::get_status(&config.url, &config.jwt, params).await?;

    let so = Success {
        jsonrpc: Some(Version::V2),
        result,
        id: Id::Num(1),
    };

    Ok(so)
}

pub async fn get_nonce(c: MethodCall, config: RemoteNodeSection) -> Result<Success, ServiceError> {
    let params = c.params.parse::<GetNonceParamsAPI>()?;
    let result = client::get_nonce(&config.url, &config.jwt, &params.account).await?;

    let so = Success {
        jsonrpc: Some(Version::V2),
        result: Value::from(result),
        id: Id::Num(1),
    };

    Ok(so)
}

#[cfg(test)]
mod tests {
    use crate::config::RemoteNodeSection;
    use crate::service::methods::get_status;
    use jsonrpc_core::{Id, MethodCall, Params, Success, Version};
    use serde_json::{json, Value};

    const TEST_URL: &str = "http://86.192.13.13:1234/rpc/v0";
    const JWT: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJBbGxvdyI6WyJyZWFkIiwid3JpdGUiLCJzaWduIiwiYWRtaW4iXX0.xK1G26jlYnAEnGLJzN1RLywghc4p4cHI6ax_6YOv0aI";

    #[tokio::test]
    async fn example_get_status_transaction() {
        let params_str = json!({ "cid_message": "bafy2bzacea2ob4bctlucgp2okbczqvk5ctx4jqjapslz57mbcmnnzyftgeqgu" });
        let params: Params =
            serde_json::from_str(&params_str.to_string()).expect("could not deserialize");

        let expected_response = Success {
            jsonrpc: Some(Version::V2),
            result: Value::from(json!({
                "To":"t1lv32q33y64xs64pnyn6om7ftirax5ikspkumwsa",
                "From":"t3wjxuftije2evjmzo2yoy5ghfe2o42mavrpmwuzooghzcxdhqjdu7kn6dvkzf4ko37w7sfnnzdzstcjmeooea",
                "Nonce":66867,
                "Value":"5000000000000000",
                "GasPrice":"0",
                "GasLimit":"1000",
                "Method":0,
                "Params":""
            })),
            id: Id::Num(1),
        };

        let mc = MethodCall {
            jsonrpc: Some(Version::V2),
            method: "get_status".to_string(),
            params,
            id: Id::Num(0),
        };

        let config = RemoteNodeSection {
            url: TEST_URL.to_string(),
            jwt: JWT.to_string(),
        };

        let status = get_status(mc, config).await.unwrap();

        println!("{:?}", status);

        assert!(status == expected_response);
    }

    #[tokio::test]
    async fn example_get_status_transaction_fail() {
        let params_str = json!({ "cid_message": "bafy2bzacedbo3svni7n2jb57exuqh4v5zvjjethf3p74zgv7yfdtczce2yu4u" });
        let params: Params =
            serde_json::from_str(&params_str.to_string()).expect("could not deserialize");

        /*let expected_response = json!({
            "jsonrpc":"2.0",
            "result":null,
            "id":1,
            "error":{
                "code":1,
                "message":
                "blockstore: block not found"
            }
        });*/

        let mc = MethodCall {
            jsonrpc: Some(Version::V2),
            method: "get_status".to_string(),
            params,
            id: Id::Num(0),
        };

        let config = RemoteNodeSection {
            url: TEST_URL.to_string(),
            jwt: JWT.to_string(),
        };

        let status = get_status(mc, config).await;

        println!("{:?}", status);

        assert!(status.is_err());
    }

    #[tokio::test]
    async fn example_get_status_transaction_fail_2() {
        let params_str = json!({ "cid_message": "bafy2bzaceaxm23epjsmh75yvzcecsrbavlmkcxnva66bkdebdcnyw3bjrc74u" });
        let params: Params =
            serde_json::from_str(&params_str.to_string()).expect("could not deserialize");

        /*let expected_response = json!({
            "jsonrpc":"2.0",
            "result":null,
            "id":1,
            "error":{
                "code":1,
                "message":"cbor input had wrong number of fields"
            }
        });*/

        let mc = MethodCall {
            jsonrpc: Some(Version::V2),
            method: "get_status".to_string(),
            params,
            id: Id::Num(0),
        };

        let config = RemoteNodeSection {
            url: TEST_URL.to_string(),
            jwt: JWT.to_string(),
        };

        let status = get_status(mc, config).await;

        println!("{:?}", status);

        assert!(status.is_err());
    }
}
