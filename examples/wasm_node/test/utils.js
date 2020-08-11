const blake = require('blakejs');

const CID_PREFIX = Buffer.from([0x01, 0x71, 0xa0, 0xe4, 0x02, 0x20]);

function getCID(message) {
  const blakeCtx = blake.blake2bInit(32);
  blake.blake2bUpdate(blakeCtx, message);
  const hash = blake.blake2bFinal(blakeCtx);
  return Buffer.concat([CID_PREFIX, hash]);
}

function getDigest(message) {
  // digest = blake2-256( prefix + blake2b-256(tx) )

  const blakeCtx = blake.blake2bInit(32);
  blake.blake2bUpdate(blakeCtx, getCID(message));
  // We want a buffer
  return Buffer.from(blake.blake2bFinal(blakeCtx));
}

function blake2b256(message) {
  const blakeCtx = blake.blake2bInit(32);
  blake.blake2bUpdate(blakeCtx, message);
  const hash = blake.blake2bFinal(blakeCtx);
  return hash;
}

module.exports = { getCID, getDigest, blake2b256 };
