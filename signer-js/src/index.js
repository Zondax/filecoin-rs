const bip39 = require('bip39');
const bip32 = require('bip32');
const cbor = require('borc');
const base32Encode = require('base32-encode');
const blake2 = require('blake2');
const secp256k1 = require('secp256k1');
const assert = require('assert');

const ExtendedKey = require('./extendedkey');
const { getDigest, getAccountFromPath, addressAsBytes, bytesToAddress, trimBuffer, tryToPrivateKeyBuffer } = require('./utils');
const { Types } = require('./constants');

function generateMnemonic() {
  // 256 so it generate 24 words
  return bip39.generateMnemonic(256);
}

function keyDeriveFromSeed(seed, path) {
  if (typeof seed === 'string') { seed = Buffer.from(seed, 'hex'); }

  const masterKey = bip32.fromSeed(seed);

  const childKey = masterKey.derivePath(path);

  let testnet = false;
  if (getAccountFromPath(path) === '1') {
    testnet = true;
  }

  return new ExtendedKey(childKey.privateKey, testnet);
}

function keyDerive(mnemonic, path, password) {
  const seed = bip39.mnemonicToSeedSync(mnemonic, password);
  return keyDeriveFromSeed(seed, path);
}

function keyRecover(privateKey, testnet) {
  // verify format and convert to buffer if needed
  privateKey = tryToPrivateKeyBuffer(privateKey);

  return new ExtendedKey(privateKey, testnet);
}

function transactionSerializeRaw(message) {
  if (!'to' in message || typeof message.to !== 'string') { throw new Error("'to' is a required field and has to be a 'string'") };
  if (!'from' in message || typeof message.from !== 'string') { throw new Error("'from' is a required field and has to be a 'string'") };
  if (!'nonce' in message || typeof message.nonce !== 'number') { throw new Error("'nonce' is a required field and has to be a 'number'") };
  if (!'value' in message || typeof message.value !== 'string') { throw new Error("'value' is a required field and has to be a 'string'") };
  if (!'gasprice' in message || typeof message.gasprice !== 'string') { throw new Error("'gasprice' is a required field and has to be a 'string'") };
  if (!'gaslimit' in message || typeof message.gaslimit !== 'number') { throw new Error("'gaslimit' is a required field and has to be a 'number'") };
  if (!'method' in message || typeof message.method !== 'number') { throw new Error("'method' is a required field and has to be a 'number'") };

  let to = addressAsBytes(message.to);
  let from = addressAsBytes(message.from);

  let valueBigInt = BigInt(message.value);
  let valueBuffer = Buffer.allocUnsafe(8);
  valueBuffer.writeBigUInt64BE(valueBigInt, 0, 8);
  let value = trimBuffer(valueBuffer);

  let gaspriceBigInt = BigInt(message.gasprice);
  let gaspriceBuffer = Buffer.allocUnsafe(8);
  gaspriceBuffer.writeBigUInt64BE(gaspriceBigInt, 0, 8);
  let gasprice = trimBuffer(gaspriceBuffer);

  let message_to_encode = [0, to, from, message.nonce, value, gasprice, message.gaslimit, message.method, Buffer.from("")];

  return cbor.encode(message_to_encode);
}

function transactionSerialize(message) {
  const raw_cbor = transactionSerializeRaw(message);
  return Buffer.from(raw_cbor).toString('hex');
}

function transactionParse(cborMessage, testnet) {
  // FIXME: Check buffer size and extra bytes
  // https://github.com/dignifiedquire/borc/issues/47
  const decoded = cbor.decodeFirst(cborMessage);

  if (decoded[0] !== 0) { throw new Error("Unsupported version") };
  if (decoded.length < 9) { throw new Error("The cbor is missing some fields... please verify you 9 fields.") };

  let message = {};

  message.to = bytesToAddress(decoded[1], testnet);
  message.from = bytesToAddress(decoded[2], testnet);
  message.nonce = decoded[3];
  message.value = decoded[4].readUIntBE(0,decoded[4].length).toString(10);
  message.gasprice = decoded[5].readUIntBE(0,decoded[5].length).toString(10);
  message.gaslimit = decoded[6];
  message.method = decoded[7];
  message.params = decoded[8].toString();

  return message;
}

function transactionSignRaw(unsignedMessage, privateKey) {
  if (typeof unsignedMessage === 'object') { unsignedMessage = transactionSerializeRaw(unsignedMessage); };
  if (typeof unsignedMessage === 'string') { unsignedMessage = Buffer.from(unsignedMessage, 'hex'); };

  // verify format and convert to buffer if needed
  privateKey = tryToPrivateKeyBuffer(privateKey);

  const messageDigest = getDigest(unsignedMessage);
  const signature = secp256k1.ecdsaSign(messageDigest, privateKey);

  let signatureRSV = Buffer.concat([signature.signature, Buffer.from([signature.recid])]);

  return signatureRSV;
}

function transactionSign(unsignedMessage, privateKey) {
  if (typeof unsignedMessage !== 'object') { throw new Error("'message' need to be an object. Cannot be under CBOR format.") };
  const signature =  transactionSignRaw(unsignedMessage, privateKey);

  let signedMessage = {};

  signedMessage.message = unsignedMessage;

  // FIXME: only support secp256k1
  signedMessage.signature = {
    data: signature.toString('base64'),
    type: Types.SECP256K1
  }

  return signedMessage;
}

function transactionSignLotus(unsignedMessage, privateKey) {
  let signedMessage = transactionSign(unsignedMessage, privateKey);

  let signedMessageJSON = JSON.stringify({
    "Message": {
      "From": signedMessage.message.from,
      "GasLimit": signedMessage.message.gaslimit,
      "GasPrice": signedMessage.message.gasprice,
      "Method": signedMessage.message.method,
      "Nonce": signedMessage.message.nonce,
      "Params": signedMessage.message.params,
      "To": signedMessage.message.to,
      "Value": signedMessage.message.value
    },
    "Signature": {
      "Data": signedMessage.signature.data,
      "Type": signedMessage.signature.type
    }
  });

  return signedMessageJSON;
}

// TODO: new function 'verifySignature(signedMessage)'; Makes more sense ?
function verifySignature(signature, message) {

  if (typeof message === 'object') { message = transactionSerializeRaw(message); };
  if (typeof message === 'string') { message = Buffer.from(message, 'hex'); };

  if (typeof signature === 'string') {
    // We should have a padding!
    if (signature.slice(-1) === '=') {
      signature = Buffer.from(signature, 'base64');
    } else {
      signature = Buffer.from(signature, 'hex');
    }
  }

  const messageDigest = getDigest(message);

  const publicKey = secp256k1.ecdsaRecover(signature.slice(0, -1), signature[64], messageDigest, false);
  const result = secp256k1.ecdsaVerify(signature.slice(0, -1), messageDigest, publicKey)

  return result;
}

module.exports = {
  generateMnemonic,
  keyDerive,
  keyDeriveFromSeed,
  keyRecover,
  transactionSerialize,
  transactionSerializeRaw,
  transactionParse,
  transactionSign,
  transactionSignLotus,
  transactionSignRaw,
  verifySignature
}
