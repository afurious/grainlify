#![cfg(test)]

//! Error Precedence Tests
//!
//! These tests verify that when multiple error conditions could apply,
//! the contract returns the highest-priority error according to the
//! documented precedence hierarchy in ERROR_PRECEDENCE.md

use crate::{
    BountyEscrowContract, BountyEscrowContractClient, DataKey, Error, Escrow, EscrowStatus,
    PauseFlags,
};
use soroban_sdk::{symbol_short, testutils::Address as _, Address, Env};

/// Helper to setup a test environment with initialized contract
fn setup_initialized_contract() -> (Env, BountyEscrowContractClient, Address, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let depositor = Address::generate(&env);

    client.init(&admin, &token);

    (env, client, admin, token, depositor)
}

#[test]
fn test_precedence_paused_over_not_initialized() {
    // Setup: Contract not initialized AND operation paused
    // Expected: FundsPaused (priority 1) over NotInitialized (priority 2)
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);
    let depositor = Address::generate(&env);

    // Set pause flag without initializing
    env.as_contract(&contract_id, || {
        env.storage().instance().set(
            &DataKey::PauseFlags,
            &PauseFlags {
                lock: true,
                release: false,
                refund: false,
            },
        );
    });

    // Try to lock funds - should fail with FundsPaused, not NotInitialized
    let result = client.try_lock_funds(&depositor, &1, &1000, &1000);
    assert_eq!(result, Err(Ok(Error::FundsPaused)));
}

#[test]
fn test_precedence_not_initialized_over_bounty_exists() {
    // Setup: Contract not initialized AND bounty would already exist
    // Expected: NotInitialized (priority 2) over BountyExists (priority 5)
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);
    let depositor = Address::generate(&env);

    // Create a bounty without initializing (simulate existing state)
    env.as_contract(&contract_id, || {
        let escrow = Escrow {
            depositor: depositor.clone(),
            amount: 1000,
            status: EscrowStatus::Locked,
            deadline: 2000,
            refund_history: soroban_sdk::vec![&env],
            remaining_amount: 1000,
        };
        env.storage()
            .persistent()
            .set(&DataKey::Escrow(1), &escrow);
    });

    // Try to lock funds with same ID - should fail with NotInitialized first
    let result = client.try_lock_funds(&depositor, &1, &1000, &2000);
    assert_eq!(result, Err(Ok(Error::NotInitialized)));
}

#[test]
fn test_precedence_bounty_exists_over_amount_validation() {
    // Setup: Bounty already exists AND amount violates policy
    // Expected: BountyExists (priority 5) over AmountBelowMinimum (priority 8)
    let (env, client, _admin, _token, depositor) = setup_initialized_contract();

    // Set amount policy
    client.set_amount_policy(&100, &10000);

    // Create an existing bounty
    let bounty_id = 1;
    env.as_contract(&client.address, || {
        let escrow = Escrow {
            depositor: depositor.clone(),
            amount: 1000,
            status: EscrowStatus::Locked,
            deadline: 2000,
            refund_history: soroban_sdk::vec![&env],
            remaining_amount: 1000,
        };
        env.storage()
            .persistent()
            .set(&DataKey::Escrow(bounty_id), &escrow);
    });

    // Try to create another bounty with same ID and amount below minimum
    let result = client.try_lock_funds(
        &depositor,
        &bounty_id, // Duplicate ID
        &50,        // Below minimum
        &2000,
    );

    // Should fail with BountyExists (priority 5)
    assert_eq!(result, Err(Ok(Error::BountyExists)));
}

#[test]
fn test_precedence_not_found_over_invalid_state() {
    // Setup: Bounty doesn't exist (so we can't check its state)
    // Expected: BountyNotFound (priority 4) before any state checks
    let (_env, client, _admin, _token, _depositor) = setup_initialized_contract();
    let contributor = Address::generate(&_env);

    // Try to release funds for non-existent bounty
    let result = client.try_release_funds(&999, &contributor);
    assert_eq!(result, Err(Ok(Error::BountyNotFound)));
}

#[test]
fn test_precedence_claim_pending_over_invalid_state() {
    // Setup: Pending claim exists AND funds in wrong state
    // Expected: ClaimPending (priority 5) over FundsNotLocked (priority 6)
    let (env, client, _admin, _token, depositor) = setup_initialized_contract();
    let contributor = Address::generate(&env);

    // Create a released bounty with pending claim
    let bounty_id = 1;
    env.as_contract(&client.address, || {
        let escrow = Escrow {
            depositor: depositor.clone(),
            amount: 1000,
            status: EscrowStatus::Released, // Already released
            deadline: 2000,
            refund_history: soroban_sdk::vec![&env],
            remaining_amount: 0,
        };
        env.storage()
            .persistent()
            .set(&DataKey::Escrow(bounty_id), &escrow);

        // Add pending claim
        let claim = crate::ClaimRecord {
            bounty_id,
            recipient: contributor.clone(),
            amount: 1000,
            expires_at: 9999,
            claimed: false,
        };
        env.storage()
            .persistent()
            .set(&DataKey::PendingClaim(bounty_id), &claim);
    });

    // Try to release - both ClaimPending and FundsNotLocked apply
    let result = client.try_release_funds(&bounty_id, &contributor);

    // Should fail with ClaimPending (priority 5) before FundsNotLocked (priority 6)
    assert_eq!(result, Err(Ok(Error::ClaimPending)));
}

#[test]
fn test_precedence_invalid_state_over_invalid_amount() {
    // Setup: Funds already released AND amount would be invalid
    // Expected: FundsNotLocked (priority 6) over InvalidAmount (priority 8)
    let (env, client, _admin, _token, depositor) = setup_initialized_contract();
    let contributor = Address::generate(&env);
    let holder = Address::generate(&env);

    // Create a released bounty
    let bounty_id = 1;
    env.as_contract(&client.address, || {
        let escrow = Escrow {
            depositor: depositor.clone(),
            amount: 1000,
            status: EscrowStatus::Released, // Already released
            deadline: 2000,
            refund_history: soroban_sdk::vec![&env],
            remaining_amount: 0,
        };
        env.storage()
            .persistent()
            .set(&DataKey::Escrow(bounty_id), &escrow);
    });

    // Issue a capability
    let capability_id = client.issue_capability(
        &holder,
        &crate::CapabilityAction::Release,
        &Some(bounty_id),
        &Some(500),
        &Some(1),
        &None,
    );

    // Try to release with capability - invalid amount (0) AND funds not locked
    let result = client.try_release_with_capability(
        &bounty_id,
        &contributor,
        &0, // Invalid amount
        &holder,
        &capability_id,
    );

    // Should fail with FundsNotLocked (priority 6), not InvalidAmount (priority 8)
    assert_eq!(result, Err(Ok(Error::FundsNotLocked)));
}

#[test]
fn test_precedence_invalid_amount_over_insufficient_funds() {
    // Setup: Invalid amount (zero) AND insufficient funds
    // Expected: InvalidAmount (priority 8) over InsufficientFunds (priority 10)
    let (env, client, _admin, _token, depositor) = setup_initialized_contract();
    let contributor = Address::generate(&env);
    let holder = Address::generate(&env);

    // Create a locked bounty with small balance
    let bounty_id = 1;
    env.as_contract(&client.address, || {
        let escrow = Escrow {
            depositor: depositor.clone(),
            amount: 100,
            status: EscrowStatus::Locked,
            deadline: 2000,
            refund_history: soroban_sdk::vec![&env],
            remaining_amount: 100,
        };
        env.storage()
            .persistent()
            .set(&DataKey::Escrow(bounty_id), &escrow);
    });

    // Issue a capability with high limit
    let capability_id = client.issue_capability(
        &holder,
        &crate::CapabilityAction::Release,
        &Some(bounty_id),
        &Some(10000), // High limit
        &Some(1),
        &None,
    );

    // Try to release with invalid amount (0) - would also be insufficient
    let result = client.try_release_with_capability(
        &bounty_id,
        &contributor,
        &0, // Invalid amount (priority 8)
        &holder,
        &capability_id,
    );

    // Should fail with InvalidAmount before checking InsufficientFunds
    assert_eq!(result, Err(Ok(Error::InvalidAmount)));
}

#[test]
fn test_precedence_amount_below_minimum_checked_in_order() {
    // Setup: Amount below minimum (no other errors)
    // Expected: AmountBelowMinimum (priority 8)
    let (_env, client, _admin, _token, depositor) = setup_initialized_contract();

    // Set amount policy
    client.set_amount_policy(&100, &10000);

    // Try to lock funds with amount below minimum
    let result = client.try_lock_funds(
        &depositor,
        &1, // New bounty
        &50, // Below minimum
        &2000,
    );

    assert_eq!(result, Err(Ok(Error::AmountBelowMinimum)));
}

#[test]
fn test_precedence_amount_above_maximum_checked_in_order() {
    // Setup: Amount above maximum (no other errors)
    // Expected: AmountAboveMaximum (priority 8)
    let (_env, client, _admin, _token, depositor) = setup_initialized_contract();

    // Set amount policy
    client.set_amount_policy(&100, &10000);

    // Try to lock funds with amount above maximum
    let result = client.try_lock_funds(
        &depositor,
        &1,     // New bounty
        &20000, // Above maximum
        &2000,
    );

    assert_eq!(result, Err(Ok(Error::AmountAboveMaximum)));
}

#[test]
fn test_precedence_comprehensive_lock_funds() {
    // Test the complete validation chain for lock_funds
    let (env, client, _admin, _token, depositor) = setup_initialized_contract();

    // 1. Pause state (priority 1) - highest priority operational check
    env.as_contract(&client.address, || {
        env.storage().instance().set(
            &DataKey::PauseFlags,
            &PauseFlags {
                lock: true,
                release: false,
                refund: false,
            },
        );
    });

    let result = client.try_lock_funds(&depositor, &1, &1000, &2000);
    assert_eq!(result, Err(Ok(Error::FundsPaused)));

    // Unpause for next tests
    env.as_contract(&client.address, || {
        env.storage().instance().set(
            &DataKey::PauseFlags,
            &PauseFlags {
                lock: false,
                release: false,
                refund: false,
            },
        );
    });

    // 2. State conflict (priority 5) - bounty already exists
    env.as_contract(&client.address, || {
        let escrow = Escrow {
            depositor: depositor.clone(),
            amount: 1000,
            status: EscrowStatus::Locked,
            deadline: 2000,
            refund_history: soroban_sdk::vec![&env],
            remaining_amount: 1000,
        };
        env.storage()
            .persistent()
            .set(&DataKey::Escrow(1), &escrow);
    });

    let result = client.try_lock_funds(&depositor, &1, &1000, &2000);
    assert_eq!(result, Err(Ok(Error::BountyExists)));

    // 3. Business logic (priority 8) - amount validation
    client.set_amount_policy(&100, &10000);
    let result = client.try_lock_funds(&depositor, &2, &50, &2000);
    assert_eq!(result, Err(Ok(Error::AmountBelowMinimum)));

    let result = client.try_lock_funds(&depositor, &2, &20000, &2000);
    assert_eq!(result, Err(Ok(Error::AmountAboveMaximum)));
}

#[test]
fn test_precedence_comprehensive_release_funds() {
    // Test the complete validation chain for release_funds
    let (env, client, _admin, _token, depositor) = setup_initialized_contract();
    let contributor = Address::generate(&env);

    // 1. Pause state (priority 1)
    env.as_contract(&client.address, || {
        env.storage().instance().set(
            &DataKey::PauseFlags,
            &PauseFlags {
                lock: false,
                release: true,
                refund: false,
            },
        );
    });

    let result = client.try_release_funds(&1, &contributor);
    assert_eq!(result, Err(Ok(Error::FundsPaused)));

    // Unpause
    env.as_contract(&client.address, || {
        env.storage().instance().set(
            &DataKey::PauseFlags,
            &PauseFlags {
                lock: false,
                release: false,
                refund: false,
            },
        );
    });

    // 2. Resource existence (priority 4)
    let result = client.try_release_funds(&999, &contributor);
    assert_eq!(result, Err(Ok(Error::BountyNotFound)));

    // 3. Create bounty and test state conflicts (priority 5)
    let bounty_id = 1;
    env.as_contract(&client.address, || {
        let escrow = Escrow {
            depositor: depositor.clone(),
            amount: 1000,
            status: EscrowStatus::Locked,
            deadline: 2000,
            refund_history: soroban_sdk::vec![&env],
            remaining_amount: 1000,
        };
        env.storage()
            .persistent()
            .set(&DataKey::Escrow(bounty_id), &escrow);

        // Add pending claim
        let claim = crate::ClaimRecord {
            bounty_id,
            recipient: contributor.clone(),
            amount: 1000,
            expires_at: 9999,
            claimed: false,
        };
        env.storage()
            .persistent()
            .set(&DataKey::PendingClaim(bounty_id), &claim);
    });

    let result = client.try_release_funds(&bounty_id, &contributor);
    assert_eq!(result, Err(Ok(Error::ClaimPending)));

    // Remove claim and test resource state (priority 6)
    env.as_contract(&client.address, || {
        env.storage()
            .persistent()
            .remove(&DataKey::PendingClaim(bounty_id));

        // Change status to Released
        let mut escrow: Escrow = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(bounty_id))
            .unwrap();
        escrow.status = EscrowStatus::Released;
        env.storage()
            .persistent()
            .set(&DataKey::Escrow(bounty_id), &escrow);
    });

    let result = client.try_release_funds(&bounty_id, &contributor);
    assert_eq!(result, Err(Ok(Error::FundsNotLocked)));
}
