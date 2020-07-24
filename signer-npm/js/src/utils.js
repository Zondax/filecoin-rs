const blake = require("blakejs");
const address = require("@openworklabs/filecoin-address")
const assert = require("assert");
const { InvalidPayloadLength, UnknownProtocolIndicator } = require("./errors")

const CID_PREFIX = Buffer.from([0x01, 0x71, 0xa0, 0xe4, 0x02, 0x20]);

function getCID(message) {
  const blakeCtx = blake.blake2bInit(32);
  blake.blake2bUpdate(blakeCtx, message);
  const hash = Buffer.from(blake.blake2bFinal(blakeCtx));
  return Buffer.concat([CID_PREFIX, hash]);
}

function getDigest(message) {
  // digest = blake2-256( prefix + blake2b-256(tx) )

  const blakeCtx = blake.blake2bInit(32);
  blake.blake2bUpdate(blakeCtx, getCID(message));
  return Buffer.from(blake.blake2bFinal(blakeCtx));
}

function getPayloadSECP256K1(uncompressedPublicKey) {
  // blake2b-160
  const blakeCtx = blake.blake2bInit(20);
  blake.blake2bUpdate(blakeCtx, uncompressedPublicKey);
  return Buffer.from(blake.blake2bFinal(blakeCtx));
}

function getAccountFromPath(path) {
  return path.split("/")[2].slice(0, -1);
}

function addressAsBytes(addressStr) {
  return Buffer.from((address.newFromString(addressStr)).str, "binary")
}

function bytesToAddress(payload, testnet) {
  let addr = new address.Address(payload)
  if ((addr.protocol() == 2 || addr.protocol() == 1) && addr.payload().length != 20) {
    throw new InvalidPayloadLength()
  }
  if (addr.protocol() == 3 && addr.payload().length != 46) {
    throw new InvalidPayloadLength()
  }
  if (payload[0] > 3) {
    throw new UnknownProtocolIndicator()
  }
  return address.encode(testnet ? 't' : 'f', addr)
}

function tryToPrivateKeyBuffer(privateKey) {
  if (typeof privateKey === "string") {
    // We should have a padding!
    if (privateKey.slice(-1) === "=") {
      privateKey = Buffer.from(privateKey, "base64");
    } else {
      privateKey = Buffer.from(privateKey, "hex");
    }
  }

  assert(privateKey.length === 32);

  return privateKey;
}

module.exports = {
  getCID,
  getDigest,
  getAccountFromPath,
  addressAsBytes,
  bytesToAddress,
  tryToPrivateKeyBuffer,
  getPayloadSECP256K1,
};
