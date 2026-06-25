# Committee Root Divergence Fix - Implementation Report

## Problem Statement

When the validator set is dynamically reorganized mid-epoch (triggered by an irregular exit or a late-inclusion activation), the committee root computed from `get_beacon_committee()` diverges between the pre-reorg and post-reorg view. This causes attestation verification to fail spuriously for validators assigned to different shard committees before and after the boundary.

## Technical Invariants & Bounds

- **Epoch length**: 32 slots (SHARD_COMMITTEE_PERIOD = 256 epochs)
- **Validator set size**: bounded by 2^19 (~524k) entries
- **Committee root**: SHA-256 over the sorted list of validator indices
- **Reorg window**: slots where state.slot % SLOTS_PER_EPOCH < 4
- **Cross-reorg attestations**: must be verifiable under both pre and post committee root

## Solution Implementation

### 1. New Modules Created

#### `src/validator/committee_assignment.rs`
Implements committee assignment tracking with reorg support:

- **`PendingReorg` struct**: Records the slot range during which a reorg is active
  - `trigger_slot`: When the reorg was triggered
  - `end_slot`: When the reorg window closes (trigger_slot + 4)

- **`CommitteeView` enum**: Represents committee state
  - `Stable(Hash256)`: Normal operation with single root
  - `Ambiguous { old_root, new_root }`: During reorg with both roots valid

- **`CommitteeAssignment` struct**: Main tracker
  - Stores current and old validator indices
  - Manages pending reorg state
  - Computes committee roots with SHA-256 over sorted indices
  - Provides ambiguous views during reorg windows

**Key Methods**:
- `trigger_reorg(slot)`: Initiates a reorg, capturing current state as "old"
- `update_validator_set(indices)`: Updates to new validator set
- `finalize_reorg(slot)`: Finalizes reorg after window closes
- `get_committee_view(slot)`: Returns appropriate view (stable or ambiguous)

#### `src/db/committee_cache.rs`
Implements committee root caching with reorg support:

- **`CommitteeCache`**: Stores committee roots per epoch
  - Maintains both stable and ambiguous entries
  - Auto-evicts old entries based on capacity
  - Supports transition from ambiguous to stable views

**Key Methods**:
- `store_stable(epoch, root)`: Store single committee root
- `store_ambiguous(epoch, old_root, new_root, end_slot)`: Store ambiguous entry
- `get_committee_view(epoch, slot)`: Retrieve view for verification
- `finalize_reorg(epoch, slot)`: Convert ambiguous to stable

### 2. Enhanced Existing Modules

#### `src/validator/validator_set.rs`
Added reorg tracking:
- `last_reorg_slot`: Tracks when last reorganization occurred
- `reorg_validator_set(slot)`: Entry point for triggering reorgs
- `active_validators()`: Returns current active validator indices

#### `src/attestation/verifier.rs`
Added committee-view-aware verification:
- `verify_attestation_with_committee_view()`: Accepts `CommitteeView` and validates attestations against either root during reorg
- `verify_attestation_with_root()`: Convenience wrapper for stable verification

### 3. Integration Tests

Created `tests/committee_reorg_test.rs` with 11 comprehensive tests:

1. **test_stable_committee_verification**: Verifies normal operation without reorg
2. **test_mid_epoch_exit_creates_ambiguous_view**: Tests irregular exit scenario
3. **test_cross_boundary_attestation_verification**: Core fix - attestations with old root verify during reorg
4. **test_late_inclusion_activation**: Tests late validator activation
5. **test_committee_cache_reorg_handling**: Cache behavior during reorg
6. **test_attestation_verification_fails_with_wrong_root**: Security - wrong roots still fail
7. **test_multiple_reorgs_in_epoch**: Edge case of multiple reorgs
8. **test_reorg_window_boundaries**: Precise boundary condition testing
9. **test_validator_set_integration**: Integration with ValidatorSet
10. **test_epoch_boundary_reorg**: Reorg at epoch boundary
11. **test_attestation_partial_committee**: Partial attestations during reorg

## Test Results

### Unit Tests (32 passed)
```
running 32 tests
test attestation_core::attestation::aggregator::tests::... (all passed)
test db::committee_cache::tests::... (all passed)
test validator::committee_assignment::tests::... (all passed)
test slashing_core::slashing::tests::... (all passed)

test result: ok. 32 passed; 0 failed; 0 ignored
```

### Integration Tests (11 passed)
```
running 11 tests
test test_stable_committee_verification ... ok
test test_mid_epoch_exit_creates_ambiguous_view ... ok
test test_cross_boundary_attestation_verification ... ok
test test_late_inclusion_activation ... ok
test test_committee_cache_reorg_handling ... ok
test test_attestation_verification_fails_with_wrong_root ... ok
test test_multiple_reorgs_in_epoch ... ok
test test_reorg_window_boundaries ... ok
test test_validator_set_integration ... ok
test test_epoch_boundary_reorg ... ok
test test_attestation_partial_committee ... ok

test result: ok. 11 passed; 0 failed; 0 ignored
```

### Full Test Suite
All 163 tests pass across all modules (lib tests, integration tests, and existing test suites).

## How It Works

### Normal Operation (Stable Committee)
```
Epoch 100, Slot 3200
Validators: [10, 20, 30, 40]
CommitteeView: Stable(root_A)
Attestation verification: Must match root_A
```

### During Reorg Window
```
Epoch 100, Slot 3203: Validator 40 exits irregularly
1. trigger_reorg(3203) called
2. Old validators [10, 20, 30, 40] captured
3. New validators [10, 20, 30, 50] set
4. CommitteeView: Ambiguous { old_root: root_A, new_root: root_B }

Slot 3204-3206: Reorg window active
- Attestations with root_A: ACCEPTED ✓
- Attestations with root_B: ACCEPTED ✓
- Attestations with wrong root: REJECTED ✗

Slot 3207: finalize_reorg(3207) called
CommitteeView: Stable(root_B)
- Only root_B accepted
```

### Key Security Properties

1. **No spurious failures**: Validators using pre-reorg committee root can still verify
2. **Time-bounded ambiguity**: Ambiguous period limited to 4 slots
3. **Deterministic finalization**: Automatic transition to stable state
4. **Wrong root rejection**: Invalid roots still fail verification
5. **No replay attacks**: Domain separation maintained throughout

## Files Modified/Created

### Created
- `src/validator/committee_assignment.rs` (239 lines)
- `src/db/committee_cache.rs` (236 lines)
- `src/db/mod.rs` (3 lines)
- `tests/committee_reorg_test.rs` (463 lines)
- `COMMITTEE_REORG_FIX_REPORT.md` (this file)

### Modified
- `src/validator/mod.rs`: Added committee_assignment module
- `src/validator/validator_set.rs`: Added reorg tracking
- `src/attestation/verifier.rs`: Added committee-view-aware verification
- `src/lib.rs`: Added db module

## Performance Considerations

- **Committee root computation**: O(n log n) for sorting n validators + O(n) for hashing
- **Cache lookup**: O(log E) for epoch lookup in BTreeMap
- **Cache eviction**: O(1) amortized with LRU-style eviction
- **Memory overhead**: ~256 epochs cached by default (~27 hours of history)

## Conclusion

The implementation successfully resolves the committee root divergence issue by:

1. Tracking validator set changes through reorganizations
2. Maintaining dual committee roots during transition windows
3. Allowing attestation verification against either root during the reorg period
4. Automatically finalizing to a single root after the window closes

All 163 tests pass, demonstrating that the fix is complete, correct, and doesn't break existing functionality.
