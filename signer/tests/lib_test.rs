use std::convert::TryFrom;
use std::str::FromStr;

use bip39::{Language, Seed};
use bls_signatures::Serialize;
use forest_address::Address;
use forest_encoding::blake2b_256;
use forest_encoding::to_vec;
use num_bigint_chainsafe::BigInt;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use rayon::prelude::*;

use extras::paych;

use filecoin_signer::api::{
    MessageParams, MessageTxAPI, SignatureAPI, SignedMessageAPI, UnsignedMessageAPI,
};
use filecoin_signer::signature::{Signature, SignatureBLS};
use filecoin_signer::{
    approve_multisig_message, cancel_multisig_message, collect_pymtchan, create_multisig,
    create_pymtchan, create_voucher, get_cid, key_derive, key_derive_from_seed,
    key_generate_mnemonic, key_recover, proposal_multisig_message, serialize_params,
    settle_pymtchan, sign_voucher, transaction_parse, transaction_serialize, transaction_sign,
    transaction_sign_raw, update_pymtchan, verify_aggregated_signature, verify_signature,
    verify_voucher_signature, CborBuffer, Mnemonic, PrivateKey,
};

const BLS_PUBKEY: &str = "ade28c91045e89a0dcdb49d5ed0d62a4f02d78a96dbd406a4f9d37a1cd2fb5c29058def79b01b4d1556ade74ffc07904";
const BLS_PRIVATEKEY: &str = "0x7Y0GGX92MeWBF9mcWuR5EYPxe2dy60r8XIQOD31BI=";

// NOTE: not the same transaction used in other tests.
const EXAMPLE_UNSIGNED_MESSAGE: &str = r#"
    {
        "to": "t17uoq6tp427uzv7fztkbsnn64iwotfrristwpryy",
        "from": "t1d2xrzcslx7xlbbylc5c3d5lvandqw4iwl6epxba",
        "nonce": 1,
        "value": "100000",
        "gaslimit": 1,
        "gasfeecap": "1",
        "gaspremium": "1",
        "method": 0,
        "params": ""
    }"#;

const EXAMPLE_CBOR_DATA: &str =
    "8a005501fd1d0f4dfcd7e99afcb99a8326b7dc459d32c62855011eaf1c8a4bbfeeb0870b1745b1f57503470b71160144000186a01961a84200014200010040";

/* signed message :
82                                      # array(2)
   8A                                   # array(10)
      00                                # unsigned(0)
      55                                # bytes(21)
         01FD1D0F4DFCD7E99AFCB99A8326B7DC459D32C628 # "\x01\xFD\x1D\x0FM\xFC\xD7\xE9\x9A\xFC\xB9\x9A\x83&\xB7\xDCE\x9D2\xC6("
      55                                # bytes(21)
         011EAF1C8A4BBFEEB0870B1745B1F57503470B7116 # "\x01\x1E\xAF\x1C\x8AK\xBF\xEE\xB0\x87\v\x17E\xB1\xF5u\x03G\vq\x16"
      01                                # unsigned(1)
      44                                # bytes(4)
         000186A0                       # "\x00\x01\x86\xA0"
      19 09C4                           # unsigned(2500)
      42                                # bytes(2)
         0001                           # "\x00\x01"
      42                                # bytes(2)
         0001                           # "\x00\x01"
      00                                # unsigned(0)
      40                                # bytes(0)
                                        # ""
   58 42                                # bytes(66)
      0106398485060CA2A4DEB97027F518F45569360C3873A4303926FA6909A7299D4C55883463120836358FF3396882EE0DC2CF15961BD495CDFB3DE1EE2E8BD3768E01 # "\x01\x069\x84\x85\x06\f\xA2\xA4\xDE\xB9p'\xF5\x18\xF4Ui6\f8s\xA409&\xFAi\t\xA7)\x9DLU\x884c\x12\b65\x8F\xF39h\x82\xEE\r\xC2\xCF\x15\x96\e\xD4\x95\xCD\xFB=\xE1\xEE.\x8B\xD3v\x8E\x01"
*/

const SIGNED_MESSAGE_CBOR: &str =
    "828a005501fd1d0f4dfcd7e99afcb99a8326b7dc459d32c62855011eaf1c8a4bbfeeb0870b1745b1f57503470b71160144000186a01909c4420001420001004058420106398485060ca2a4deb97027f518f45569360c3873a4303926fa6909a7299d4c55883463120836358ff3396882ee0dc2cf15961bd495cdfb3de1ee2e8bd3768e01";

const EXAMPLE_PRIVATE_KEY: &str = "8VcW07ADswS4BV2cxi5rnIadVsyTDDhY1NfDH19T8Uo=";

#[test]
fn decode_key() {
    let pk = PrivateKey::try_from(EXAMPLE_PRIVATE_KEY.to_string()).unwrap();
    assert_eq!(base64::encode(&pk.0), EXAMPLE_PRIVATE_KEY);
}

#[test]
fn generate_mnemonic() {
    let mnemonic = key_generate_mnemonic().expect("could not generate mnemonic");
    println!("{}", mnemonic.0);

    let word_count = mnemonic.0.split_ascii_whitespace().count();
    assert_eq!(word_count, 24)
}

#[test]
fn derive_key() {
    let mnemonic = "equip will roof matter pink blind book anxiety banner elbow sun young";

    let extended_key = key_derive(mnemonic, "m/44'/461'/0/0/0", "").unwrap();

    assert_eq!(
        base64::encode(&extended_key.private_key.0),
        EXAMPLE_PRIVATE_KEY
    );
}

#[test]
fn derive_key_password() {
    let mnemonic = "equip will roof matter pink blind book anxiety banner elbow sun young";

    let m = bip39::Mnemonic::from_phrase(&mnemonic.to_string(), Language::English).unwrap();

    let seed = Seed::new(&m, "password");

    let extended_key_expected = key_derive_from_seed(seed.as_bytes(), "m/44'/461'/0/0/0").unwrap();

    let extended_key = key_derive(mnemonic, "m/44'/461'/0/0/0", "password").unwrap();

    assert_eq!(
        base64::encode(&extended_key.private_key.0),
        base64::encode(&extended_key_expected.private_key.0)
    );
}

#[test]
fn derive_key_from_seed() {
    let mnemonic = Mnemonic(
        "equip will roof matter pink blind book anxiety banner elbow sun young".to_string(),
    );

    let mnemonic = bip39::Mnemonic::from_phrase(&mnemonic.0, Language::English).unwrap();

    let seed = Seed::new(&mnemonic, "");

    let extended_key = key_derive_from_seed(seed.as_bytes(), "m/44'/461'/0/0/0").unwrap();

    assert_eq!(
        base64::encode(&extended_key.private_key.0),
        EXAMPLE_PRIVATE_KEY
    );
}

#[test]
fn test_key_recover_testnet() {
    let private_key = PrivateKey::try_from(EXAMPLE_PRIVATE_KEY.to_string()).unwrap();
    let testnet = true;

    let recovered_key = key_recover(&private_key, testnet).unwrap();

    assert_eq!(
        base64::encode(&recovered_key.private_key.0),
        EXAMPLE_PRIVATE_KEY
    );

    assert_eq!(
        &recovered_key.address,
        "t1d2xrzcslx7xlbbylc5c3d5lvandqw4iwl6epxba"
    );
}

#[test]
fn test_key_recover_mainnet() {
    let private_key = PrivateKey::try_from(EXAMPLE_PRIVATE_KEY.to_string()).unwrap();
    let testnet = false;

    let recovered_key = key_recover(&private_key, testnet).unwrap();

    assert_eq!(
        base64::encode(&recovered_key.private_key.0),
        EXAMPLE_PRIVATE_KEY
    );

    assert_eq!(
        &recovered_key.address,
        "f1d2xrzcslx7xlbbylc5c3d5lvandqw4iwl6epxba"
    );
}

#[test]
fn parse_unsigned_transaction() {
    let cbor_data = CborBuffer(hex::decode(EXAMPLE_CBOR_DATA).unwrap());

    let unsigned_tx = transaction_parse(&cbor_data, true).expect("FIX ME");
    let to = match unsigned_tx {
        MessageTxAPI::UnsignedMessageAPI(tx) => tx.to,
        MessageTxAPI::SignedMessageAPI(_) => panic!("Should be a Unsigned Message!"),
    };

    println!("{}", to);
    assert_eq!(to, "t17uoq6tp427uzv7fztkbsnn64iwotfrristwpryy".to_string());
}

#[test]
fn parse_signed_transaction() {
    let cbor_data = CborBuffer(hex::decode(SIGNED_MESSAGE_CBOR).unwrap());

    let signed_tx = transaction_parse(&cbor_data, true).expect("Could not parse");
    let signature = match signed_tx {
        MessageTxAPI::UnsignedMessageAPI(_) => panic!("Should be a Signed Message!"),
        MessageTxAPI::SignedMessageAPI(tx) => tx.signature,
    };

    assert_eq!(
        hex::encode(&signature.data),
        "06398485060ca2a4deb97027f518f45569360c3873a4303926fa6909a7299d4c55883463120836358ff3396882ee0dc2cf15961bd495cdfb3de1ee2e8bd3768e01".to_string()
    );
}

#[test]
fn parse_transaction_with_network() {
    let cbor_data = CborBuffer(hex::decode(EXAMPLE_CBOR_DATA).unwrap());

    let unsigned_tx_mainnet = transaction_parse(&cbor_data, false).expect("Could not parse");
    let (to, from) = match unsigned_tx_mainnet {
        MessageTxAPI::UnsignedMessageAPI(tx) => (tx.to, tx.from),
        MessageTxAPI::SignedMessageAPI(_) => panic!("Should be a Unsigned Message!"),
    };

    println!("{}", to);
    assert_eq!(to, "f17uoq6tp427uzv7fztkbsnn64iwotfrristwpryy".to_string());
    assert_eq!(
        from,
        "f1d2xrzcslx7xlbbylc5c3d5lvandqw4iwl6epxba".to_string()
    );
}

#[test]
fn parse_transaction_with_network_testnet() {
    let cbor_data = CborBuffer(hex::decode(EXAMPLE_CBOR_DATA).unwrap());

    let unsigned_tx_testnet = transaction_parse(&cbor_data, true).expect("Could not parse");
    let (to, from) = match unsigned_tx_testnet {
        MessageTxAPI::UnsignedMessageAPI(tx) => (tx.to, tx.from),
        MessageTxAPI::SignedMessageAPI(_) => panic!("Should be a Unsigned Message!"),
    };

    println!("{}", to);
    assert_eq!(to, "t17uoq6tp427uzv7fztkbsnn64iwotfrristwpryy".to_string());
    assert_eq!(
        from,
        "t1d2xrzcslx7xlbbylc5c3d5lvandqw4iwl6epxba".to_string()
    );
}

#[test]
fn parse_transaction_signed_with_network() {
    let cbor_data = CborBuffer(hex::decode(SIGNED_MESSAGE_CBOR).unwrap());

    let signed_tx_mainnet = transaction_parse(&cbor_data, false).expect("Could not parse");
    let (to, from) = match signed_tx_mainnet {
        MessageTxAPI::UnsignedMessageAPI(_) => panic!("Should be a Signed Message!"),
        MessageTxAPI::SignedMessageAPI(tx) => (tx.message.to, tx.message.from),
    };

    println!("{}", to);
    assert_eq!(to, "f17uoq6tp427uzv7fztkbsnn64iwotfrristwpryy".to_string());
    assert_eq!(
        from,
        "f1d2xrzcslx7xlbbylc5c3d5lvandqw4iwl6epxba".to_string()
    );
}

#[test]
fn parse_transaction_signed_with_network_testnet() {
    let cbor_data = CborBuffer(hex::decode(SIGNED_MESSAGE_CBOR).unwrap());

    let signed_tx_testnet = transaction_parse(&cbor_data, true).expect("Could not parse");
    let (to, from) = match signed_tx_testnet {
        MessageTxAPI::UnsignedMessageAPI(_) => panic!("Should be a Signed Message!"),
        MessageTxAPI::SignedMessageAPI(tx) => (tx.message.to, tx.message.from),
    };

    assert_eq!(to, "t17uoq6tp427uzv7fztkbsnn64iwotfrristwpryy".to_string());
    assert_eq!(
        from,
        "t1d2xrzcslx7xlbbylc5c3d5lvandqw4iwl6epxba".to_string()
    );
}

#[test]
fn verify_invalid_signature() {
    // Path 44'/461'/0/0/0
    let private_key = PrivateKey::try_from(EXAMPLE_PRIVATE_KEY.to_string()).unwrap();
    let message_user_api: UnsignedMessageAPI = serde_json::from_str(EXAMPLE_UNSIGNED_MESSAGE)
        .expect("Could not serialize unsigned message");

    // Sign
    let signature = transaction_sign_raw(&message_user_api, &private_key).unwrap();

    // Verify
    let message = forest_message::UnsignedMessage::try_from(&message_user_api)
        .expect("Could not serialize unsigned message");
    let message_cbor = CborBuffer(to_vec(&message).unwrap());

    let valid_signature = verify_signature(&signature, &message_cbor);
    assert!(valid_signature.unwrap());

    // Tampered signature and look if it valid
    let mut sig = signature.as_bytes();
    sig[5] = 0x01;
    sig[34] = 0x00;

    let tampered_signature = Signature::try_from(sig).expect("FIX ME");

    let valid_signature = verify_signature(&tampered_signature, &message_cbor);
    assert!(valid_signature.is_err() || !valid_signature.unwrap());
}

#[test]
fn sign_bls_transaction() {
    // Get address
    let bls_pubkey = hex::decode(BLS_PUBKEY).unwrap();
    let bls_address = Address::new_bls(bls_pubkey.as_slice()).unwrap();

    // Get BLS private key
    let bls_key = PrivateKey::try_from(BLS_PRIVATEKEY.to_string()).unwrap();

    println!("{}", bls_address.to_string());

    // Prepare message with BLS address
    let message = UnsignedMessageAPI {
        to: "t17uoq6tp427uzv7fztkbsnn64iwotfrristwpryy".to_string(),
        from: bls_address.to_string(),
        nonce: 1,
        value: "100000".to_string(),
        gas_limit: 25000,
        gas_fee_cap: "2500".to_string(),
        gas_premium: "2500".to_string(),
        method: 0,
        params: "".to_string(),
    };

    let raw_sig = transaction_sign_raw(&message, &bls_key).unwrap();
    let sig = bls_signatures::Signature::from_bytes(&raw_sig.as_bytes()).expect("FIX ME");

    let bls_pk = bls_signatures::PublicKey::from_bytes(&hex::decode(BLS_PUBKEY).unwrap()).unwrap();

    let message_cbor = transaction_serialize(&message).expect("FIX ME");

    assert!(bls_pk.verify(sig, &message_cbor));
}

#[test]
fn test_verify_aggregated_signature() {
    // sign 3 messages
    let num_messages = 3;

    let mut rng = ChaCha8Rng::seed_from_u64(12);

    // generate private keys
    let private_keys: Vec<_> = (0..num_messages)
        .map(|_| bls_signatures::PrivateKey::generate(&mut rng))
        .collect();

    // generate messages
    let messages: Vec<UnsignedMessageAPI> = (0..num_messages)
        .map(|i| {
            //Prepare transaction
            let bls_public_key = private_keys[i].public_key();
            let bls_address = Address::new_bls(&bls_public_key.as_bytes()).unwrap();

            UnsignedMessageAPI {
                to: "t17uoq6tp427uzv7fztkbsnn64iwotfrristwpryy".to_string(),
                from: bls_address.to_string(),
                nonce: 1,
                value: "100000".to_string(),
                gas_limit: 25000,
                gas_fee_cap: "2500".to_string(),
                gas_premium: "2500".to_string(),
                method: 0,
                params: "".to_string(),
            }
        })
        .collect();

    // sign messages
    let sigs: Vec<bls_signatures::Signature>;
    sigs = messages
        .par_iter()
        .zip(private_keys.par_iter())
        .map(|(message, pk)| {
            let private_key = PrivateKey::try_from(pk.as_bytes()).expect("FIX ME");
            let raw_sig = transaction_sign_raw(message, &private_key).unwrap();

            bls_signatures::Serialize::from_bytes(&raw_sig.as_bytes()).expect("FIX ME")
        })
        .collect::<Vec<bls_signatures::Signature>>();

    // serialize messages
    let cbor_messages: Vec<CborBuffer>;
    cbor_messages = messages
        .par_iter()
        .map(|message| transaction_serialize(message).unwrap())
        .collect::<Vec<CborBuffer>>();

    let aggregated_signature = bls_signatures::aggregate(&sigs).expect("FIX ME");

    let sig = SignatureBLS::try_from(aggregated_signature.as_bytes()).expect("FIX ME");

    assert!(verify_aggregated_signature(&sig, &cbor_messages[..]).unwrap());
}

#[test]
fn payment_channel_creation_bls_signing() {
    let from_key = "8niW4fUBoKNo3GMDVfWu0oari11js4t1QpwXVBpEpFA=".to_string();
    let _from_address =
        "t3smdzzt2fbrzalmfi5rskc3tc6wpwcj2zbgyu5engqtkkzrxteg2oyqpukqzrhqqfvzqadh7mtqye443liejq";
    let bls_key = PrivateKey::try_from(from_key).unwrap();
    let from_pkey = "93079ccf450c7205b0a8ec64a16e62f59f61275909b14e91a684d4acc6f321b4ec41f4543313c205ae60019fec9c304e";

    let pch_create = serde_json::json!(
    {
        "to": "t01",           // INIT_ACTOR_ADDR
        "from": "t3smdzzt2fbrzalmfi5rskc3tc6wpwcj2zbgyu5engqtkkzrxteg2oyqpukqzrhqqfvzqadh7mtqye443liejq",
        "nonce": 1,
        "value": "1",
        "gaslimit": 200000000,
        "gasfeecap": "2500",
        "gaspremium": "2500",
        "method": 2,           // extras::MethodInit::Exec
        "params": "gtgqWBkAAVUAFGZpbC8xL3BheW1lbnRjaGFubmVsWEqCWDEDkwecz0UMcgWwqOxkoW5i9Z9hJ1kJsU6RpoTUrMbzIbTsQfRUMxPCBa5gAZ/snDBOVQElRUfDOAbbTJ6ACbjr2cTS5fIBgg=="
    });

    let pch_create_message_api = create_pymtchan(
        "t3smdzzt2fbrzalmfi5rskc3tc6wpwcj2zbgyu5engqtkkzrxteg2oyqpukqzrhqqfvzqadh7mtqye443liejq"
            .to_string(),
        "t1evcupqzya3nuzhuabg4oxwoe2ls7eamcu3uw4cy".to_string(),
        "1".to_string(),
        1,
        200000000,
        "2500".to_string(),
        "2500".to_string(),
    )
    .unwrap();

    let pch_create_message_expected: UnsignedMessageAPI =
        serde_json::from_value(pch_create).unwrap();

    assert_eq!(
        serde_json::to_string(&pch_create_message_expected).unwrap(),
        serde_json::to_string(&pch_create_message_api).unwrap()
    );

    // First check transaction_serialize() in creating an unsigned message
    let result = transaction_serialize(&pch_create_message_api).unwrap();

    // Now check that we can generate a correct signature
    let sig = transaction_sign_raw(&pch_create_message_api, &bls_key).unwrap();

    let bls_pkey = bls_signatures::PublicKey::from_bytes(&hex::decode(from_pkey).unwrap()).unwrap();

    let bls_sig = bls_signatures::Serialize::from_bytes(&sig.as_bytes()).expect("FIX ME");

    assert!(bls_pkey.verify(bls_sig, &result));
}

// This example reverses the to/from addresses compared with
// previous test.
const PYMTCHAN_EXAMPLE_UNSIGNED_MSG: &str = r#"
    {
        "to": "t01",
        "from": "t1evcupqzya3nuzhuabg4oxwoe2ls7eamcu3uw4cy",
        "nonce": 1,
        "value": "1",
        "gaslimit": 1000000,
        "gasfeecap": "2500",
        "gaspremium": "2500",
        "method": 2,
        "params": "gtgqWBkAAVUAFGZpbC8xL3BheW1lbnRjaGFubmVsWEqCVQElRUfDOAbbTJ6ACbjr2cTS5fIBglgxA5MHnM9FDHIFsKjsZKFuYvWfYSdZCbFOkaaE1KzG8yG07EH0VDMTwgWuYAGf7JwwTg=="
    }"#;

#[test]
fn payment_channel_creation_secp256k1_signing() {
    let from_key = "+UXJi0663hCExYMxZVb9J+wKyFWhhX51jnG7WXkeAw0=".to_string();
    let _from_pkey = "254547c33806db4c9e8009b8ebd9c4d2e5f20182";
    let privkey = PrivateKey::try_from(from_key).unwrap();

    let _pch_create_message_unsigned = serde_json::json!(PYMTCHAN_EXAMPLE_UNSIGNED_MSG);
    let pch_create_message_api: UnsignedMessageAPI =
        serde_json::from_str(PYMTCHAN_EXAMPLE_UNSIGNED_MSG)
            .expect("Could not serialize unsigned message");
    // TODO:  ^^^ this is an error, these lines are duplicated.  First one should have called create_pymtchan()

    let signed_message_result = transaction_sign(&pch_create_message_api, &privkey).unwrap();
    // TODO:  how do I check the signature of a transaction_sign() result

    // Check the raw bytes match the test vector cbor
    let _cbor_result_unsigned_msg = transaction_serialize(&signed_message_result.message).unwrap();
}

const PYMTCHAN_UPDATE_EXAMPLE_UNSIGNED_MSG: &str = r#"
{
    "to": "t2oajfrgjjllncvbxx4shzbxy3nnegsrnnk3tq2tq",
    "from": "t1gsu6clgzpcrjxclicnsva5bty3r65hnkqpd4jaq",
    "nonce": 1,
    "value": "0",
    "gaslimit": 200000000,
    "gasfeecap": "2500",
    "gaspremium": "2500",
    "method": 2,
    "params": "g4tVAnASWJkpWtoqhvfkj5DfG2tIaUWtAABA9gABQgABAIBYQwEBchH8MsS6EHe1a9/gW2lb30YbwD++F+2BRIUTUykZz9U6nt+nGfb41Yf0sy2NfaToz8Il/GmDtnGCepu/ns7nNwFAQA=="
}"#;

#[test]
fn payment_channel_update() {
    use forest_crypto::signature::Signature;

    let from_key = "Is8RE05W1aR6Xyk4IbpVA71sU2ibVQQgle80rjs8U8E=".to_string();
    let _from_pkey = "34a9e12cd978a29b89681365507433c6e3ee9daa"; // from base32decode("gsu6clgzpcrjxclicnsva5bty3r65hnk")
    let _pch_addr_hex = "70125899295ada2a86f7e48f90df1b6b486945ad"; // from base32decode("oajfrgjjllncvbxx4shzbxy3nnegsrnn")
    let privkey = PrivateKey::try_from(from_key).unwrap();

    let sig_decoded = hex::decode("017211fc32c4ba1077b56bdfe05b695bdf461bc03fbe17ed81448513532919cfd53a9edfa719f6f8d587f4b32d8d7da4e8cfc225fc6983b671827a9bbf9ecee73701").unwrap();
    let sig = Signature::new_secp256k1(sig_decoded);

    let pch = Address::from_str("t2oajfrgjjllncvbxx4shzbxy3nnegsrnnk3tq2tq").unwrap();

    let sv = paych::SignedVoucher {
        channel_addr: pch,
        time_lock_min: 0,
        time_lock_max: 0,
        secret_pre_image: Vec::new(),
        extra: Option::<paych::ModVerifyParams>::None,
        lane: 0,
        nonce: 1,
        amount: BigInt::parse_bytes(b"1", 10).unwrap(),
        min_settle_height: 0,
        merges: vec![],
        signature: Some(sig),
    };

    let sv_base64 = base64::encode(to_vec(&sv).unwrap());

    let pch_update_message_unsigned_api = update_pymtchan(
        "t2oajfrgjjllncvbxx4shzbxy3nnegsrnnk3tq2tq".to_string(),
        "t1gsu6clgzpcrjxclicnsva5bty3r65hnkqpd4jaq".to_string(),
        sv_base64,
        1,
        200000000,
        "2500".to_string(),
        "2500".to_string(),
    )
    .unwrap();

    let pch_update_message_unsigned_expected: UnsignedMessageAPI =
        serde_json::from_str(PYMTCHAN_UPDATE_EXAMPLE_UNSIGNED_MSG)
            .expect("Could not serialize unsigned message");

    assert_eq!(
        serde_json::to_string(&pch_update_message_unsigned_expected).unwrap(),
        serde_json::to_string(&pch_update_message_unsigned_api).unwrap()
    );

    // Sign
    let signature = transaction_sign_raw(&pch_update_message_unsigned_api, &privkey).unwrap();

    // Verify
    let message = forest_message::UnsignedMessage::try_from(&pch_update_message_unsigned_api)
        .expect("Could not serialize unsigned message");
    let message_cbor = CborBuffer(to_vec(&message).unwrap());

    let valid_signature = verify_signature(&signature, &message_cbor);
    assert!(valid_signature.unwrap());
}

const PYMTCHAN_SETTLE_EXAMPLE_UNSIGNED_MSG: &str = r#"
{
    "to": "t2oajfrgjjllncvbxx4shzbxy3nnegsrnnk3tq2tq",
    "from": "t1gsu6clgzpcrjxclicnsva5bty3r65hnkqpd4jaq",
    "nonce": 1,
    "value": "0",
    "gaslimit": 20000000,
    "gasfeecap": "2500",
    "gaspremium": "2500",
    "method": 3,
    "params": ""
}"#;

#[test]
fn payment_channel_settle() {
    let from_key = "Is8RE05W1aR6Xyk4IbpVA71sU2ibVQQgle80rjs8U8E=".to_string();
    let _from_pkey = "34a9e12cd978a29b89681365507433c6e3ee9daa"; // from base32decode("gsu6clgzpcrjxclicnsva5bty3r65hnk")
    let _pch_addr_hex = "70125899295ada2a86f7e48f90df1b6b486945ad"; // from base32decode("oajfrgjjllncvbxx4shzbxy3nnegsrnn")
    let privkey = PrivateKey::try_from(from_key).unwrap();

    let pch_settle_message_unsigned_api = settle_pymtchan(
        "t2oajfrgjjllncvbxx4shzbxy3nnegsrnnk3tq2tq".to_string(),
        "t1gsu6clgzpcrjxclicnsva5bty3r65hnkqpd4jaq".to_string(),
        1,
        20000000,
        "2500".to_string(),
        "2500".to_string(),
    )
    .unwrap();

    let pch_settle_message_unsigned_expected: UnsignedMessageAPI =
        serde_json::from_str(PYMTCHAN_SETTLE_EXAMPLE_UNSIGNED_MSG)
            .expect("Could not serialize unsigned message");

    assert_eq!(
        serde_json::to_string(&pch_settle_message_unsigned_expected).unwrap(),
        serde_json::to_string(&pch_settle_message_unsigned_api).unwrap()
    );

    // Sign
    let signature = transaction_sign_raw(&pch_settle_message_unsigned_api, &privkey).unwrap();

    // Verify
    let message = forest_message::UnsignedMessage::try_from(&pch_settle_message_unsigned_api)
        .expect("Could not serialize unsigned message");
    let message_cbor = CborBuffer(to_vec(&message).unwrap());

    let valid_signature = verify_signature(&signature, &message_cbor);
    assert!(valid_signature.unwrap());
}

const PYMTCHAN_COLLECT_EXAMPLE_UNSIGNED_MSG: &str = r#"
{
    "to": "t2oajfrgjjllncvbxx4shzbxy3nnegsrnnk3tq2tq",
    "from": "t1gsu6clgzpcrjxclicnsva5bty3r65hnkqpd4jaq",
    "nonce": 1,
    "value": "0",
    "gaslimit": 20000000,
    "gasfeecap": "2500",
    "gaspremium": "2500",
    "method": 4,
    "params": ""
}"#;

#[test]
fn payment_channel_collect() {
    let from_key = "Is8RE05W1aR6Xyk4IbpVA71sU2ibVQQgle80rjs8U8E=".to_string();
    let _from_pkey = "34a9e12cd978a29b89681365507433c6e3ee9daa"; // from base32decode("gsu6clgzpcrjxclicnsva5bty3r65hnk")
    let _pch_addr_hex = "70125899295ada2a86f7e48f90df1b6b486945ad"; // from base32decode("oajfrgjjllncvbxx4shzbxy3nnegsrnn")
    let privkey = PrivateKey::try_from(from_key).unwrap();

    let pch_collect_message_unsigned_api = collect_pymtchan(
        "t2oajfrgjjllncvbxx4shzbxy3nnegsrnnk3tq2tq".to_string(),
        "t1gsu6clgzpcrjxclicnsva5bty3r65hnkqpd4jaq".to_string(),
        1,
        20000000,
        "2500".to_string(),
        "2500".to_string(),
    )
    .unwrap();

    let pch_collect_message_unsigned_expected: UnsignedMessageAPI =
        serde_json::from_str(PYMTCHAN_COLLECT_EXAMPLE_UNSIGNED_MSG)
            .expect("Could not serialize unsigned message");

    assert_eq!(
        serde_json::to_string(&pch_collect_message_unsigned_expected).unwrap(),
        serde_json::to_string(&pch_collect_message_unsigned_api).unwrap()
    );

    // Sign
    let signature = transaction_sign_raw(&pch_collect_message_unsigned_api, &privkey).unwrap();

    // Verify
    let message = forest_message::UnsignedMessage::try_from(&pch_collect_message_unsigned_api)
        .expect("Could not serialize unsigned message");
    let message_cbor = CborBuffer(to_vec(&message).unwrap());

    let valid_signature = verify_signature(&signature, &message_cbor);
    assert!(valid_signature.unwrap());
}

#[test]
fn test_sign_voucher() {
    let mnemonic = "equip will roof matter pink blind book anxiety banner elbow sun young";
    let extended_key = key_derive(mnemonic, "m/44'/461'/0/0/0", "").unwrap();

    let voucher = create_voucher(
        "t24acjqhdetck7irsvmn2p6jpuwnouzjxuoa22rva".to_string(),
        0,
        0,
        "10000".to_string(),
        1,
        1,
        0,
    )
    .unwrap();

    let signed_voucher = sign_voucher(voucher, &extended_key.private_key).unwrap();

    assert_eq!(signed_voucher, "i1UC4ASYHGSYlfRGVWN0/yX0s11MpvQAAED2AQFDACcQAIBYQgFRD/3a1fsyc7TLRUgeQ5BAPhB1rDuVt1qvDuwccTODWCJ+OAe4R/+HIGH9pgBYjrghhA4JdgJugTWfzFflbOGSAA==");
}

#[test]
fn support_multisig_create() {
    let constructor_params = serde_json::json!({
        "signers": ["t1d2xrzcslx7xlbbylc5c3d5lvandqw4iwl6epxba", "t137sjdbgunloi7couiy4l5nc7pd6k2jmq32vizpy"],
        "num_approvals_threshold": 1,
        "unlock_duration": 0
    });

    let constructor_params_expected: MessageParams =
        serde_json::from_value(constructor_params).unwrap();

    let exec_params = serde_json::json!({
        "code_cid": "fil/1/multisig",
        "constructor_params": base64::encode(serialize_params(constructor_params_expected).unwrap())
    });

    let exec_params_expected: MessageParams = serde_json::from_value(exec_params).unwrap();

    let multisig_create = serde_json::json!(
    {
        "to": "t01",
        "from": "t1d2xrzcslx7xlbbylc5c3d5lvandqw4iwl6epxba",
        "nonce": 1,
        "value": "1000",
        "gaslimit": 1000000,
        "gasfeecap": "2500",
        "gaspremium": "2500",
        "method": 2,
        "params": base64::encode(serialize_params(exec_params_expected).unwrap()),
    });

    let multisig_create_message_api = create_multisig(
        "t1d2xrzcslx7xlbbylc5c3d5lvandqw4iwl6epxba".to_string(),
        vec![
            "t1d2xrzcslx7xlbbylc5c3d5lvandqw4iwl6epxba".to_string(),
            "t137sjdbgunloi7couiy4l5nc7pd6k2jmq32vizpy".to_string(),
        ],
        "1000".to_string(),
        1,
        1,
        0,
        1000000,
        "2500".to_string(),
        "2500".to_string(),
    )
    .unwrap();

    let multisig_create_message_expected: UnsignedMessageAPI =
        serde_json::from_value(multisig_create).unwrap();

    assert_eq!(
        serde_json::to_string(&multisig_create_message_expected).unwrap(),
        serde_json::to_string(&multisig_create_message_api).unwrap()
    );

    let result = transaction_serialize(&multisig_create_message_api).unwrap();

    println!("{}", hex::encode(&result));

    assert_eq!(
        hex::encode(&result),
        "8a0042000155011eaf1c8a4bbfeeb0870b1745b1f57503470b711601430003e81a000f4240430009c4430009c402584982d82a53000155000e66696c2f312f6d756c74697369675830838255011eaf1c8a4bbfeeb0870b1745b1f57503470b71165501dfe49184d46adc8f89d44638beb45f78fcad25900100"
    );
}

#[test]
fn support_multisig_propose_message() {
    let proposal_params = serde_json::json!({
        "to": "t137sjdbgunloi7couiy4l5nc7pd6k2jmq32vizpy",
        "value": "1000",
        "method": 0,
        "params": "",
    });

    let proposal_params_expected: MessageParams = serde_json::from_value(proposal_params).unwrap();

    let multisig_proposal = serde_json::json!(
    {
        "to": "t01004",
        "from": "t1d2xrzcslx7xlbbylc5c3d5lvandqw4iwl6epxba",
        "nonce": 1,
        "value": "0",
        "gaslimit": 1000000,
        "gasfeecap": "2500",
        "gaspremium": "2500",
        "method": 2,
        "params": base64::encode(serialize_params(proposal_params_expected).unwrap())
    });

    let multisig_proposal_message_api = proposal_multisig_message(
        "t01004".to_string(),
        "t137sjdbgunloi7couiy4l5nc7pd6k2jmq32vizpy".to_string(),
        "t1d2xrzcslx7xlbbylc5c3d5lvandqw4iwl6epxba".to_string(),
        "1000".to_string(),
        1,
        1000000,
        "2500".to_string(),
        "2500".to_string(),
    )
    .unwrap();

    let multisig_proposal_message_expected: UnsignedMessageAPI =
        serde_json::from_value(multisig_proposal).unwrap();

    assert_eq!(
        serde_json::to_string(&multisig_proposal_message_expected).unwrap(),
        serde_json::to_string(&multisig_proposal_message_api).unwrap()
    );

    let result = transaction_serialize(&multisig_proposal_message_api).unwrap();

    println!("{}", hex::encode(&result));

    assert_eq!(
        hex::encode(&result),
        "8a004300ec0755011eaf1c8a4bbfeeb0870b1745b1f57503470b711601401a000f4240430009c4430009c402581d845501dfe49184d46adc8f89d44638beb45f78fcad2590430003e80040"
    );
}

#[test]
fn support_multisig_approve_message() {
    let proposal_params = serde_json::json!({
        "requester": "t1d2xrzcslx7xlbbylc5c3d5lvandqw4iwl6epxba",
        "to": "t137sjdbgunloi7couiy4l5nc7pd6k2jmq32vizpy",
        "value": "1000",
        "method": 0,
        "params": "",
    });

    let proposal_params_expected: MessageParams = serde_json::from_value(proposal_params).unwrap();

    let proposal_hash = blake2b_256(serialize_params(proposal_params_expected).unwrap().as_ref());

    let approval_params = serde_json::json!({
        "txn_id": 1234,
        "proposal_hash_data": base64::encode(proposal_hash),
    });

    let approval_params_expected: MessageParams = serde_json::from_value(approval_params).unwrap();

    let multisig_approval = serde_json::json!(
    {
        "to": "t01004",
        "from": "t1d2xrzcslx7xlbbylc5c3d5lvandqw4iwl6epxba",
        "nonce": 1,
        "value": "0",
        "gaslimit": 1000000,
        "gasfeecap": "2500",
        "gaspremium": "2500",
        "method": 3,
        "params": base64::encode(serialize_params(approval_params_expected).unwrap()),
    });

    let multisig_approval_message_api = approve_multisig_message(
        "t01004".to_string(),
        1234,
        "t1d2xrzcslx7xlbbylc5c3d5lvandqw4iwl6epxba".to_string(),
        "t137sjdbgunloi7couiy4l5nc7pd6k2jmq32vizpy".to_string(),
        "1000".to_string(),
        "t1d2xrzcslx7xlbbylc5c3d5lvandqw4iwl6epxba".to_string(),
        1,
        1000000,
        "2500".to_string(),
        "2500".to_string(),
    )
    .unwrap();

    let multisig_approval_message_expected: UnsignedMessageAPI =
        serde_json::from_value(multisig_approval).unwrap();

    assert_eq!(
        serde_json::to_string(&multisig_approval_message_expected).unwrap(),
        serde_json::to_string(&multisig_approval_message_api).unwrap()
    );

    let result = transaction_serialize(&multisig_approval_message_api).unwrap();

    println!("{}", hex::encode(&result));

    assert_eq!(
        hex::encode(&result),
        "8a004300ec0755011eaf1c8a4bbfeeb0870b1745b1f57503470b711601401a000f4240430009c4430009c4035826821904d25820f8acf2652972f009aeaa1d9b61cfcd86702b3093c19c3049604f19db8cb378f3"
    );
}

#[test]
fn support_multisig_cancel_message() {
    let proposal_params = serde_json::json!({
        "requester": "t1d2xrzcslx7xlbbylc5c3d5lvandqw4iwl6epxba",
        "to": "t137sjdbgunloi7couiy4l5nc7pd6k2jmq32vizpy",
        "value": "1000",
        "method": 0,
        "params": "",
    });

    let proposal_params_expected: MessageParams = serde_json::from_value(proposal_params).unwrap();

    let proposal_hash = blake2b_256(serialize_params(proposal_params_expected).unwrap().as_ref());

    let cancel_params = serde_json::json!({
        "txn_id": 1234,
        "proposal_hash_data": base64::encode(proposal_hash),
    });

    let cancel_params_expected: MessageParams = serde_json::from_value(cancel_params).unwrap();

    let multisig_cancel = serde_json::json!(
    {
        "to": "t01004",
        "from": "t1d2xrzcslx7xlbbylc5c3d5lvandqw4iwl6epxba",
        "nonce": 1,
        "value": "0",
        "gaslimit": 1000000,
        "gasfeecap": "2500",
        "gaspremium": "2500",
        "method": 4,
        "params": base64::encode(serialize_params(cancel_params_expected).unwrap()),
    });

    let multisig_cancel_message_api = cancel_multisig_message(
        "t01004".to_string(),
        1234,
        "t1d2xrzcslx7xlbbylc5c3d5lvandqw4iwl6epxba".to_string(),
        "t137sjdbgunloi7couiy4l5nc7pd6k2jmq32vizpy".to_string(),
        "1000".to_string(),
        "t1d2xrzcslx7xlbbylc5c3d5lvandqw4iwl6epxba".to_string(),
        1,
        1000000,
        "2500".to_string(),
        "2500".to_string(),
    )
    .unwrap();

    let multisig_cancel_message_expected: UnsignedMessageAPI =
        serde_json::from_value(multisig_cancel).unwrap();

    assert_eq!(
        serde_json::to_string(&multisig_cancel_message_expected).unwrap(),
        serde_json::to_string(&multisig_cancel_message_api).unwrap()
    );

    let result = transaction_serialize(&multisig_cancel_message_api).unwrap();

    println!("{}", hex::encode(&result));

    assert_eq!(
        hex::encode(&result),
        "8a004300ec0755011eaf1c8a4bbfeeb0870b1745b1f57503470b711601401a000f4240430009c4430009c4045826821904d25820f8acf2652972f009aeaa1d9b61cfcd86702b3093c19c3049604f19db8cb378f3"
    );
}

#[test]
fn test_verify_voucher_signature() {
    let voucher_base64_string = "i0MA8gcAAED2AAFEAAGGoACAWEIBayRmYQQCatrELBc2rwfu0jJk0EmVr+eVccDsThtM1ZVzkrC53a6qVgrgFkB8OHoiZSlNmW/nmCU7G2POhEeo2gE=".to_string();
    let address_signer = "t1d2xrzcslx7xlbbylc5c3d5lvandqw4iwl6epxba".to_string();

    let result = verify_voucher_signature(voucher_base64_string, address_signer).expect("FIX ME");

    assert!(result);
}

#[test]
fn test_get_cid() {
    let expected_cid = "bafy2bzacebaiinljwwctblf7czp4zxwhz4747z6tpricgn5cumd4xhebftcvu".to_string();
    let message = UnsignedMessageAPI {
        to: "t17uoq6tp427uzv7fztkbsnn64iwotfrristwpryy".to_string(),
        from: "t1d2xrzcslx7xlbbylc5c3d5lvandqw4iwl6epxba".to_string(),
        nonce: 1,
        value: "100000".to_string(),
        gas_limit: 2500000,
        gas_fee_cap: "1".to_string(),
        gas_premium: "1".to_string(),
        method: 0,
        params: "".to_string(),
    };
    let signature = SignatureAPI{
        sig_type: 1,
        data: base64::decode("0wRrFJZFIVh8m0JD+f5C55YrxD6YAWtCXWYihrPTKdMfgMhYAy86MVhs43hSLXnV+47UReRIe8qFdHRJqFlreAE=".to_string()).unwrap(),
    };

    let signed_message_api = SignedMessageAPI { message, signature };

    let cid = get_cid(signed_message_api).unwrap();

    assert_eq!(cid, expected_cid);
}
