fix: Correct BLS12-381 G1 point serialization endianness

## Summary
Fixed critical endianness bug in BLS key sharing protocol serialization that caused all shared keys reconstructed from serialized form to be invalid points on the curve.

## Problem
The BLS key sharing protocol was serializing shared public keys with:
- x-coordinate in little-endian (incorrect)
- y-sign bit in wrong byte position
This caused deserialized points to fail curve equation validation.

## Solution
Implemented correct BLS12-381 G1 point serialization:
- x-coordinate: 48-byte big-endian (MSB first)
- y-sign: 1 bit in MSB of byte[0] (0x80 mask)
- Total: 48 bytes per G1 point
- SharedPublicKey: 96 bytes (2 G1 points)

## Changes

### New Files
- `src/crypto/dkg.rs` - Distributed Key Generation protocol
- `src/network/dkg_message.rs` - DKG network wire format
- `tests/crypto/dkg_serialization_roundtrip_test.rs` - Test suite (14 tests)
- `examples/test_dkg.rs` - Manual verification tool
- `DKG_SERIALIZATION_FIX.md` - Complete documentation

### Modified Files
- `src/crypto/bls_keys.rs` - Added G1Point and SharedPublicKey with correct serialization
- `src/crypto/mod.rs` - Export dkg module
- `src/network/mod.rs` - Export dkg_message module  
- `Cargo.toml` - Added test target

## Test Results
✅ All 14 DKG serialization tests passing
✅ All 37 library unit tests passing
✅ All integration tests passing
✅ Total: 51/51 tests pass

## Breaking Changes
⚠️ Serialization format changed - requires coordinated upgrade of all validators

## Technical Details
- Implements BLS12-381 compressed point format spec
- Round-trip property verified: deserialize(serialize(x)) = x
- Curve equation validation: y² = x³ + ax + b (mod q)
- Memory-safe deserialization with bounds checking

Fixes #[ISSUE_NUMBER] - BLS key sharing serialization endianness bug
