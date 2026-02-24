#![cfg(test)]

extern crate std;

use soroban_sdk::{
    testutils::{Address as _, Events},
    Address, BytesN, Env, Vec as SorobanVec,
};

use crate::{GrainlifyContract, GrainlifyContractClient};
use super::WASM;

// ============================================================================
// Test Helpers
// ============================================================================

/// Helper to create a test environment with initialized contract
fn setup_test_contract(env: &Env) -> (GrainlifyContractClient, Address) {
    let admin = Address::generate(env);
    let contract_id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(env, &contract_id);
    
    env.mock_all_auths();
    client.init_admin(&admin);
    
    (client, admin)
}

/// Helper to upload a mock "new version" WASM
/// In real scenarios, this would be the actual new contract WASM
fn upload_mock_new_wasm(env: &Env) -> BytesN<32> {
    // For testing, we'll use the same WASM but treat it as "v2"
    // In production, this would be a different compiled WASM
    env.deployer().upload_contract_wasm(WASM)
}

/// Helper to upload the current WASM (for rollback testing)
fn upload_current_wasm(env: &Env) -> BytesN<32> {
    env.deployer().upload_contract_wasm(WASM)
}

// ============================================================================
// TEST 1: Basic Upgrade and Rollback Cycle
// ============================================================================

#[test]
fn test_upgrade_then_rollback_preserves_state() {
    let env = Env::default();
    let (client, _admin) = setup_test_contract(&env);
    
    // Verify initial state
    let initial_version = client.get_version();
    assert_eq!(initial_version, 2, "Initial version should be 2");
    
    // Upload both WASM versions
    let current_wasm = upload_current_wasm(&env);
    let new_wasm = upload_mock_new_wasm(&env);
    
    // Perform upgrade
    client.upgrade(&new_wasm);
    
    // Verify upgrade succeeded (version tracking)
    let previous_version = client.get_previous_version();
    assert_eq!(
        previous_version,
        Some(initial_version),
        "Previous version should be stored"
    );
    
    // Rollback to original WASM
    client.upgrade(&current_wasm);
    
    // Verify rollback succeeded
    let rolled_back_version = client.get_version();
    assert_eq!(
        rolled_back_version, initial_version,
        "Version should be restored after rollback"
    );
}

// ============================================================================
// TEST 2: Multiple Upgrade/Rollback Cycles
// ============================================================================

#[test]
fn test_multiple_upgrade_rollback_cycles() {
    let env = Env::default();
    let (client, _admin) = setup_test_contract(&env);
    
    let wasm_v1 = upload_current_wasm(&env);
    let wasm_v2 = upload_mock_new_wasm(&env);
    
    let initial_version = client.get_version();
    
    // Perform 3 upgrade/rollback cycles
    for cycle in 0..3 {
        // Upgrade
        client.upgrade(&wasm_v2);
        let prev = client.get_previous_version();
        assert!(
            prev.is_some(),
            "Cycle {}: Previous version should be tracked",
            cycle
        );
        
        // Rollback
        client.upgrade(&wasm_v1);
        let current = client.get_version();
        assert_eq!(
            current, initial_version,
            "Cycle {}: Version should be consistent after rollback",
            cycle
        );
    }
}

// ============================================================================
// TEST 3: WASM Hash Reuse Without Re-upload
// ============================================================================

#[test]
fn test_wasm_hash_reuse_without_reuploading() {
    let env = Env::default();
    let (client, _admin) = setup_test_contract(&env);
    
    // Upload WASMs once
    let wasm_v1 = upload_current_wasm(&env);
    let wasm_v2 = upload_mock_new_wasm(&env);
    
    // First upgrade
    client.upgrade(&wasm_v2);
    
    // Rollback using cached hash (no re-upload)
    client.upgrade(&wasm_v1);
    
    // Second upgrade using same cached hash
    client.upgrade(&wasm_v2);
    
    // Second rollback using same cached hash
    client.upgrade(&wasm_v1);
    
    // Verify final state is correct
    assert_eq!(client.get_version(), 2, "Final version should be 2");
}

// ============================================================================
// TEST 4: Upgrade Events Are Emitted
// ============================================================================

#[test]
fn test_upgrade_and_rollback_emit_events() {
    let env = Env::default();
    let (client, _admin) = setup_test_contract(&env);
    
    let wasm_v1 = upload_current_wasm(&env);
    let wasm_v2 = upload_mock_new_wasm(&env);
    
    let initial_event_count = env.events().all().len();
    
    // Upgrade
    client.upgrade(&wasm_v2);
    let events_after_upgrade = env.events().all();
    assert!(
        events_after_upgrade.len() > initial_event_count,
        "Upgrade should emit events"
    );
    
    // Rollback
    client.upgrade(&wasm_v1);
    let events_after_rollback = env.events().all();
    assert!(
        events_after_rollback.len() > events_after_upgrade.len(),
        "Rollback should emit additional events"
    );
}

// ============================================================================
// TEST 5: Only Admin Can Upgrade/Rollback
// ============================================================================

#[test]
#[should_panic]
fn test_non_admin_cannot_upgrade() {
    let env = Env::default();
    let admin = Address::generate(&env);
    
    let contract_id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(&env, &contract_id);
    
    // Initialize with admin
    env.mock_all_auths();
    client.init_admin(&admin);
    
    // Clear mocked auths
    env.mock_auths(&[]);
    
    // Try to upgrade as non-admin (should panic)
    let new_wasm = upload_mock_new_wasm(&env);
    client.upgrade(&new_wasm);
}

// ============================================================================
// TEST 6: Version Tracking Across Upgrades
// ============================================================================

#[test]
fn test_version_tracking_across_upgrades() {
    let env = Env::default();
    let (client, _admin) = setup_test_contract(&env);
    
    let initial_version = client.get_version();
    assert_eq!(initial_version, 2);
    
    let wasm_v2 = upload_mock_new_wasm(&env);
    
    // Upgrade
    client.upgrade(&wasm_v2);
    
    // Check previous version is tracked
    let prev = client.get_previous_version();
    assert_eq!(prev, Some(initial_version));
    
    // Update version number
    client.set_version(&3);
    assert_eq!(client.get_version(), 3);
    
    // Upgrade again
    let wasm_v3 = upload_mock_new_wasm(&env);
    client.upgrade(&wasm_v3);
    
    // Previous version should now be 3
    let prev2 = client.get_previous_version();
    assert_eq!(prev2, Some(3));
}

// ============================================================================
// TEST 7: Rollback Preserves Admin
// ============================================================================

#[test]
fn test_rollback_preserves_admin() {
    let env = Env::default();
    let (client, _admin) = setup_test_contract(&env);
    
    let wasm_v1 = upload_current_wasm(&env);
    let wasm_v2 = upload_mock_new_wasm(&env);
    
    // Upgrade
    client.upgrade(&wasm_v2);
    
    // Rollback
    client.upgrade(&wasm_v1);
    
    // Verify admin can still perform operations
    client.set_version(&5);
    assert_eq!(client.get_version(), 5);
}

// ============================================================================
// TEST 8: State Persistence Across Upgrade/Rollback
// ============================================================================

#[test]
fn test_state_persistence_across_upgrade_rollback() {
    let env = Env::default();
    let (client, _admin) = setup_test_contract(&env);
    
    // Set a custom version
    client.set_version(&10);
    assert_eq!(client.get_version(), 10);
    
    let wasm_v1 = upload_current_wasm(&env);
    let wasm_v2 = upload_mock_new_wasm(&env);
    
    // Upgrade
    client.upgrade(&wasm_v2);
    
    // Version should still be accessible
    assert_eq!(client.get_version(), 10);
    
    // Rollback
    client.upgrade(&wasm_v1);
    
    // Version should still be 10
    assert_eq!(client.get_version(), 10);
}

// ============================================================================
// TEST 9: Migration State Survives Rollback
// ============================================================================

#[test]
fn test_migration_state_survives_rollback() {
    let env = Env::default();
    let (client, _admin) = setup_test_contract(&env);
    
    // Perform migration
    let migration_hash = BytesN::from_array(&env, &[1u8; 32]);
    client.migrate(&3, &migration_hash);
    
    let migration_state = client.get_migration_state();
    assert!(migration_state.is_some());
    let state = migration_state.unwrap();
    assert_eq!(state.to_version, 3);
    
    // Upgrade and rollback
    let wasm_v1 = upload_current_wasm(&env);
    let wasm_v2 = upload_mock_new_wasm(&env);
    
    client.upgrade(&wasm_v2);
    client.upgrade(&wasm_v1);
    
    // Migration state should still exist
    let migration_state_after = client.get_migration_state();
    assert!(migration_state_after.is_some());
    let state_after = migration_state_after.unwrap();
    assert_eq!(state_after.to_version, 3);
    assert_eq!(state_after.migration_hash, migration_hash);
}

// ============================================================================
// TEST 10: Rollback After Failed Migration
// ============================================================================

#[test]
fn test_rollback_after_failed_migration() {
    let env = Env::default();
    let (client, _admin) = setup_test_contract(&env);
    
    let wasm_v1 = upload_current_wasm(&env);
    let wasm_v2 = upload_mock_new_wasm(&env);
    
    // Upgrade to v2
    client.upgrade(&wasm_v2);
    
    // Try invalid migration (should fail)
    let migration_hash = BytesN::from_array(&env, &[2u8; 32]);
    let result = client.try_migrate(&1, &migration_hash);
    assert!(result.is_err(), "Migration to lower version should fail");
    
    // Rollback should still work
    client.upgrade(&wasm_v1);
    assert_eq!(client.get_version(), 2);
}

// ============================================================================
// TEST 11: Event History Validation
// ============================================================================

#[test]
fn test_event_history_validation() {
    let env = Env::default();
    let (client, _admin) = setup_test_contract(&env);
    
    let wasm_v1 = upload_current_wasm(&env);
    let wasm_v2 = upload_mock_new_wasm(&env);
    
    // Track events through upgrade cycle
    let events_initial = env.events().all();
    
    client.upgrade(&wasm_v2);
    let events_after_upgrade = env.events().all();
    
    client.upgrade(&wasm_v1);
    let events_after_rollback = env.events().all();
    
    // Verify event progression
    assert!(events_after_upgrade.len() > events_initial.len());
    assert!(events_after_rollback.len() > events_after_upgrade.len());
}

// ============================================================================
// TEST 12: Version Number Consistency
// ============================================================================

#[test]
fn test_version_number_consistency() {
    let env = Env::default();
    let (client, _admin) = setup_test_contract(&env);
    
    // Test semantic version functions
    let version = client.get_version();
    assert_eq!(version, 2);
    
    let semver = client.get_version_semver_string();
    assert_eq!(semver, soroban_sdk::String::from_str(&env, "2.0.0"));
    
    let numeric = client.get_version_numeric_encoded();
    assert_eq!(numeric, 20000);
    
    // Upgrade and verify consistency
    let wasm_v2 = upload_mock_new_wasm(&env);
    client.upgrade(&wasm_v2);
    
    // Version functions should still work
    let version_after = client.get_version();
    assert_eq!(version_after, 2);
}

// ============================================================================
// TEST 13: Multisig Upgrade and Rollback
// ============================================================================

#[test]
fn test_multisig_upgrade_and_rollback() {
    let env = Env::default();
    env.mock_all_auths();
    
    let signer1 = Address::generate(&env);
    let signer2 = Address::generate(&env);
    let signer3 = Address::generate(&env);
    
    let mut signers = SorobanVec::new(&env);
    signers.push_back(signer1.clone());
    signers.push_back(signer2.clone());
    signers.push_back(signer3.clone());
    
    let contract_id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(&env, &contract_id);
    
    // Initialize with multisig (2 of 3)
    client.init(&signers, &2);
    
    let wasm_v1 = upload_current_wasm(&env);
    let wasm_v2 = upload_mock_new_wasm(&env);
    
    // Propose upgrade
    let proposal_id = client.propose_upgrade(&signer1, &wasm_v2);
    
    // Approve with 2 signers
    client.approve_upgrade(&proposal_id, &signer1);
    client.approve_upgrade(&proposal_id, &signer2);
    
    // Execute upgrade
    client.execute_upgrade(&proposal_id);
    
    // Propose rollback
    let rollback_proposal_id = client.propose_upgrade(&signer2, &wasm_v1);
    
    // Approve rollback
    client.approve_upgrade(&rollback_proposal_id, &signer2);
    client.approve_upgrade(&rollback_proposal_id, &signer3);
    
    // Execute rollback
    client.execute_upgrade(&rollback_proposal_id);
    
    // Verify rollback succeeded
    assert_eq!(client.get_version(), 2);
}

// ============================================================================
// TEST 14: Upgrade with Migration and Rollback
// ============================================================================

#[test]
fn test_upgrade_with_migration_and_rollback() {
    let env = Env::default();
    let (client, _admin) = setup_test_contract(&env);
    
    let wasm_v1 = upload_current_wasm(&env);
    let wasm_v2 = upload_mock_new_wasm(&env);
    
    // Initial state
    assert_eq!(client.get_version(), 2);
    
    // Upgrade
    client.upgrade(&wasm_v2);
    
    // Migrate to v3
    let migration_hash = BytesN::from_array(&env, &[3u8; 32]);
    client.migrate(&3, &migration_hash);
    assert_eq!(client.get_version(), 3);
    
    // Rollback WASM
    client.upgrade(&wasm_v1);
    
    // Version should still be 3 (migration state persists)
    assert_eq!(client.get_version(), 3);
    
    // Migration state should be intact
    let state = client.get_migration_state().unwrap();
    assert_eq!(state.to_version, 3);
}

// ============================================================================
// TEST 15: Rapid Upgrade/Rollback Stress Test
// ============================================================================

#[test]
fn test_rapid_upgrade_rollback_stress() {
    let env = Env::default();
    let (client, _admin) = setup_test_contract(&env);
    
    let wasm_v1 = upload_current_wasm(&env);
    let wasm_v2 = upload_mock_new_wasm(&env);
    
    // Perform 10 rapid upgrade/rollback cycles
    for i in 0..10 {
        client.upgrade(&wasm_v2);
        client.upgrade(&wasm_v1);
        
        // Verify state is consistent
        assert_eq!(
            client.get_version(),
            2,
            "Iteration {}: Version should remain consistent",
            i
        );
    }
}
