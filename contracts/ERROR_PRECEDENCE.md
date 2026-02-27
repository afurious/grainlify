# Smart Contract Error Precedence and Deterministic Ordering

## Overview

This document defines the canonical ordering of validation checks across all Grainlify smart contracts (escrow and core contracts). When multiple error conditions could apply to a single transaction, contracts MUST return errors in the priority order defined below. This ensures deterministic, predictable behavior and improves debuggability.

## Rationale

Deterministic error ordering provides:
- **Predictability**: Same inputs always produce the same error
- **Debuggability**: Developers know which error to fix first  
- **Testability**: Tests can reliably assert specific error codes
- **Consistency**: All contracts follow the same validation pattern
- **Upgrade Safety**: Error ordering remains stable across versions

## Error Precedence Hierarchy

Validation checks MUST be performed in this order (highest to lowest priority):

### 1. Pause State (Priority: CRITICAL)
- **Errors**: `FundsPaused`
- **Rationale**: Operational pause flags prevent state changes during maintenance or incidents
- **Examples**: `check_paused()` returning true for the operation

### 2. Contract Initialization (Priority: HIGH)
- **Errors**: `NotInitialized`, `AlreadyInitialized`
- **Rationale**: Contract must be in valid initialized state before any operations
- **Examples**: Missing admin, missing token address

### 3. Authorization & Authentication (Priority: HIGH)
- **Errors**: `Unauthorized`, `require_auth()` failures
- **Rationale**: Caller must be authorized before validating their request
- **Examples**: Non-admin calling admin-only function, missing signature

### 4. Resource Existence (Priority: HIGH)
- **Errors**: `BountyNotFound`, `CapabilityNotFound`, `ProposalNotFound`
- **Rationale**: Referenced resources must exist before validating operations on them
- **Examples**: Operating on non-existent bounty ID

### 5. State Conflicts (Priority: MEDIUM)
- **Errors**: `BountyExists`, `DuplicateBountyId`, `AlreadyApproved`, `ClaimPending`
- **Rationale**: Operation conflicts with existing state
- **Examples**: Creating bounty with existing ID, pending claim blocks refund

### 6. Resource State (Priority: MEDIUM)
- **Errors**: `FundsNotLocked`, `AlreadyExecuted`, `CapabilityRevoked`, `CapabilityExpired`
- **Rationale**: Resource must be in correct state for the operation
- **Examples**: Trying to release funds that are already released

### 7. Capability Validation (Priority: MEDIUM)
- **Errors**: `CapabilityActionMismatch`, `CapabilityAmountExceeded`, `CapabilityUsesExhausted`, `CapabilityExceedsAuthority`
- **Rationale**: Delegated permissions must be validated after resource state
- **Examples**: Capability doesn't allow the requested action

### 8. Business Logic Constraints (Priority: MEDIUM)
- **Errors**: `InvalidAmount`, `InvalidDeadline`, `AmountBelowMinimum`, `AmountAboveMaximum`, `InvalidThreshold`, `InvalidFeeRate`, `InvalidAssetId`
- **Rationale**: Input parameters must meet business rules
- **Examples**: Amount is zero, deadline in the past, invalid asset ID

### 9. Dependency & Precondition Checks (Priority: LOW)
- **Errors**: `ThresholdNotMet`, `DeadlineNotPassed`, `RefundNotApproved`
- **Rationale**: External conditions or dependencies must be satisfied
- **Examples**: Multisig threshold not reached, deadline hasn't passed yet

### 10. Resource Availability (Priority: LOW)
- **Errors**: `InsufficientFunds`, `InsufficientStake`
- **Rationale**: Check resource availability after all other validations
- **Examples**: Contract balance too low for payout

### 11. Batch Operation Errors (Priority: LOW)
- **Errors**: `InvalidBatchSize`, `BatchSizeMismatch`
- **Rationale**: Batch-specific validation after individual item validation
- **Examples**: Empty batch, mismatched array lengths

## Standard Validation Pattern

All contract entry points SHOULD follow this pattern:

```rust
pub fn operation(env: Env, params: ...) -> Result<(), Error> {
    // 1. Pause state
    if Self::check_paused(&env, symbol_short!("operation")) {
        return Err(Error::FundsPaused);
    }
    
    // 2. Initialization
    if !env.storage().instance().has(&DataKey::Admin) {
        return Err(Error::NotInitialized);
    }
    
    // 3. Authorization
    caller.require_auth();
    
    // 4. Resource existence
    if !env.storage().persistent().has(&DataKey::Resource(id)) {
        return Err(Error::ResourceNotFound);
    }
    
    // 5. State conflicts
    if env.storage().persistent().has(&DataKey::Duplicate(id)) {
        return Err(Error::AlreadyExists);
    }
    
    // 6. Resource state
    let resource = load_resource(&env, id);
    if resource.status != ExpectedStatus {
        return Err(Error::InvalidState);
    }
    
    // 7. Capability validation (if using capabilities)
    validate_capability(&env, capability_id, action, amount)?;
    
    // 8. Business logic constraints
    if amount <= 0 {
        return Err(Error::InvalidAmount);
    }
    if deadline < env.ledger().timestamp() {
        return Err(Error::InvalidDeadline);
    }
    
    // 9. Dependencies & preconditions
    if !precondition_met(&env) {
        return Err(Error::PreconditionNotMet);
    }
    
    // 10. Resource availability
    if balance < required_amount {
        return Err(Error::InsufficientFunds);
    }
    
    // 11. Batch validations (if applicable)
    if items.len() == 0 {
        return Err(Error::InvalidBatchSize);
    }
    
    // Effects and interactions (CEI pattern)
    // ...
    
    Ok(())
}
```

## Contract-Specific Error Mappings

### Bounty Escrow Contract

```rust
pub enum Error {
    AlreadyInitialized = 1,      // Priority: 2 (Initialization)
    NotInitialized = 2,           // Priority: 2 (Initialization)
    BountyExists = 3,             // Priority: 5 (State Conflicts)
    BountyNotFound = 4,           // Priority: 4 (Resource Existence)
    FundsNotLocked = 5,           // Priority: 6 (Resource State)
    DeadlineNotPassed = 6,        // Priority: 9 (Dependencies)
    Unauthorized = 7,             // Priority: 3 (Authorization)
    InvalidFeeRate = 8,           // Priority: 8 (Business Logic)
    FeeRecipientNotSet = 9,       // Priority: 2 (Initialization)
    InvalidBatchSize = 10,        // Priority: 11 (Batch Operations)
    BatchSizeMismatch = 11,       // Priority: 11 (Batch Operations)
    DuplicateBountyId = 12,       // Priority: 5 (State Conflicts)
    InvalidAmount = 13,           // Priority: 8 (Business Logic)
    InvalidDeadline = 14,         // Priority: 8 (Business Logic)
    InsufficientFunds = 16,       // Priority: 10 (Resource Availability)
    RefundNotApproved = 17,       // Priority: 9 (Dependencies)
    FundsPaused = 18,             // Priority: 1 (Pause State)
    AmountBelowMinimum = 19,      // Priority: 8 (Business Logic)
    AmountAboveMaximum = 20,      // Priority: 8 (Business Logic)
    NotPaused = 21,               // Priority: 1 (Pause State)
    ClaimPending = 22,            // Priority: 5 (State Conflicts)
    CapabilityNotFound = 23,      // Priority: 4 (Resource Existence)
    CapabilityExpired = 24,       // Priority: 6 (Resource State)
    CapabilityRevoked = 25,       // Priority: 6 (Resource State)
    CapabilityActionMismatch = 26, // Priority: 7 (Capability Validation)
    CapabilityAmountExceeded = 27, // Priority: 7 (Capability Validation)
    CapabilityUsesExhausted = 28,  // Priority: 7 (Capability Validation)
    CapabilityExceedsAuthority = 29, // Priority: 7 (Capability Validation)
    InvalidAssetId = 30,          // Priority: 8 (Business Logic)
}
```

## Implementation Examples

### lock_funds() - Correct Ordering

```rust
pub fn lock_funds(env: Env, depositor: Address, bounty_id: u64, amount: i128, deadline: u64) -> Result<(), Error> {
    // Priority 1: Pause state
    if Self::check_paused(&env, symbol_short!("lock")) {
        return Err(Error::FundsPaused);
    }
    
    // Priority 2: Initialization
    if !env.storage().instance().has(&DataKey::Admin) {
        return Err(Error::NotInitialized);
    }
    
    // Priority 3: Authorization
    depositor.require_auth();
    
    // Priority 5: State conflicts (bounty already exists)
    if env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
        return Err(Error::BountyExists);
    }
    
    // Priority 8: Business logic constraints (amount validation)
    if let Some((min_amount, max_amount)) = env.storage().instance().get::<DataKey, (i128, i128)>(&DataKey::AmountPolicy) {
        if amount < min_amount {
            return Err(Error::AmountBelowMinimum);
        }
        if amount > max_amount {
            return Err(Error::AmountAboveMaximum);
        }
    }
    
    // Effects and interactions...
    Ok(())
}
```

### release_funds() - Correct Ordering

```rust
pub fn release_funds(env: Env, bounty_id: u64, contributor: Address) -> Result<(), Error> {
    // Priority 1: Pause state
    if Self::check_paused(&env, symbol_short!("release")) {
        return Err(Error::FundsPaused);
    }
    
    // Priority 2: Initialization
    if !env.storage().instance().has(&DataKey::Admin) {
        return Err(Error::NotInitialized);
    }
    
    // Priority 3: Authorization
    let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
    admin.require_auth();
    
    // Priority 4: Resource existence
    if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
        return Err(Error::BountyNotFound);
    }
    
    // Priority 5: State conflicts (pending claim)
    if env.storage().persistent().has(&DataKey::PendingClaim(bounty_id)) {
        let claim: ClaimRecord = env.storage().persistent().get(&DataKey::PendingClaim(bounty_id)).unwrap();
        if !claim.claimed {
            return Err(Error::ClaimPending);
        }
    }
    
    // Priority 6: Resource state
    let escrow: Escrow = env.storage().persistent().get(&DataKey::Escrow(bounty_id)).unwrap();
    if escrow.status != EscrowStatus::Locked {
        return Err(Error::FundsNotLocked);
    }
    
    // Effects and interactions...
    Ok(())
}
```

## Testing Requirements

All contracts MUST include tests that:

1. **Multi-Error Scenarios**: Create inputs that violate multiple constraints simultaneously
2. **Precedence Verification**: Assert that the highest-priority error is returned
3. **Regression Prevention**: Lock in error ordering to prevent accidental changes

Example test structure:

```rust
#[test]
fn test_error_precedence_paused_over_not_initialized() {
    // Setup: Contract not initialized AND operation paused
    // Expected: FundsPaused (priority 1) over NotInitialized (priority 2)
}

#[test]
fn test_error_precedence_not_found_over_invalid_amount() {
    // Setup: Bounty doesn't exist AND amount is invalid
    // Expected: BountyNotFound (priority 4) over InvalidAmount (priority 8)
}
```

## Version History

- **v1.0.0** (2026-02-25): Initial error precedence documentation
  - Defined 11-level priority hierarchy
  - Documented all contract error codes with priorities
  - Established standard validation pattern

## References

- [Checks-Effects-Interactions Pattern](./ARCHITECTURE.md)
- [Reentrancy Protection](./bounty_escrow/contracts/escrow/src/reentrancy_guard.rs)
