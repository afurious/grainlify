#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, Events, Ledger},
    token, vec, Address, Env,
};

use crate::{BountyEscrowContract, BountyEscrowContractClient};

fn create_test_env() -> (Env, BountyEscrowContractClient<'static>, Address) {
    let env = Env::default();
    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    (env, client, contract_id)
}

fn create_token_contract<'a>(
    e: &'a Env,
    admin: &Address,
) -> (Address, token::Client<'a>, token::StellarAssetClient<'a>) {
    let token_id = e.register_stellar_asset_contract_v2(admin.clone());
    let token = token_id.address();
    let token_client = token::Client::new(e, &token);
    let token_admin_client = token::StellarAssetClient::new(e, &token);
    (token, token_client, token_admin_client)
}

#[test]
fn test_init_event() {
    let (env, client, _contract_id) = create_test_env();
    let _employee = Address::generate(&env);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let _depositor = Address::generate(&env);
    let _bounty_id = 1;

    env.mock_all_auths();

    // Initialize
    client.init(&admin.clone(), &token.clone());

    // Get all events emitted
    let events = env.events().all();

    // Verify the event was emitted (1 init event + 2 monitoring events)
    assert_eq!(events.len(), 3);
}

#[test]
fn test_lock_fund() {
    let (env, client, _contract_id) = create_test_env();
    let _employee = Address::generate(&env);

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let bounty_id = 1;
    let amount = 1000;
    let deadline = 10;

    env.mock_all_auths();

    // Setup token
    let token_admin = Address::generate(&env);
    let (token, _token_client, token_admin_client) = create_token_contract(&env, &token_admin);

    // Initialize
    client.init(&admin.clone(), &token.clone());

    token_admin_client.mint(&depositor, &amount);

    client.lock_funds(&depositor, &bounty_id, &amount, &deadline);

    // Get all events emitted
    let events = env.events().all();

    // Verify the event was emitted (5 original events + 4 monitoring events from init & lock_funds)
    assert_eq!(events.len(), 9);
}

#[test]
fn test_release_fund() {
    let (env, client, _contract_id) = create_test_env();

    let admin = Address::generate(&env);
    // let token = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);
    let bounty_id = 1;
    let amount = 1000;
    let deadline = 10;

    env.mock_all_auths();

    // Setup token
    let token_admin = Address::generate(&env);
    let (token, _token_client, token_admin_client) = create_token_contract(&env, &token_admin);

    // Initialize
    client.init(&admin.clone(), &token.clone());

    token_admin_client.mint(&depositor, &amount);

    client.lock_funds(&depositor, &bounty_id, &amount, &deadline);

    client.release_funds(&bounty_id, &contributor);

    // Get all events emitted
    let events = env.events().all();

    // Verify the event was emitted (7 original events + 6 monitoring events from init, lock_funds & release_funds)
    assert_eq!(events.len(), 13);
}

#[test]
#[should_panic(expected = "Error(Contract, #8)")]
fn test_lock_fund_invalid_amount() {
    let (env, client, _contract_id) = create_test_env();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let bounty_id = 1;
    let amount = 0; // Invalid amount
    let deadline = 100;

    env.mock_all_auths();

    let token_admin = Address::generate(&env);
    let (token, _token_client, _token_admin_client) = create_token_contract(&env, &token_admin);

    client.init(&admin.clone(), &token.clone());

    client.lock_funds(&depositor, &bounty_id, &amount, &deadline);
}

#[test]
#[should_panic(expected = "Error(Contract, #9)")]
fn test_lock_fund_invalid_deadline() {
    let (env, client, _contract_id) = create_test_env();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let bounty_id = 1;
    let amount = 1000;
    let deadline = 0; // Past deadline (default timestamp is 0, so 0 <= 0)

    env.mock_all_auths();

    let token_admin = Address::generate(&env);
    let (token, _token_client, token_admin_client) = create_token_contract(&env, &token_admin);

    client.init(&admin.clone(), &token.clone());
    token_admin_client.mint(&depositor, &amount);

    client.lock_funds(&depositor, &bounty_id, &amount, &deadline);
}

// ============================================================================
// Integration Tests: Batch Operations
// ============================================================================

#[test]
fn test_batch_lock_funds() {
    let (env, client, _contract_id) = create_test_env();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let (token, _token_client, token_admin_client) = create_token_contract(&env, &token_admin);

    client.init(&admin, &token);

    // Mint tokens for batch operations
    let total_amount = 5000i128;
    token_admin_client.mint(&depositor, &total_amount);

    // Create batch lock items
    let mut items = vec![&env];
    items.push_back(crate::LockFundsItem {
        bounty_id: 1,
        depositor: depositor.clone(),
        amount: 1000,
        deadline: 100,
    });
    items.push_back(crate::LockFundsItem {
        bounty_id: 2,
        depositor: depositor.clone(),
        amount: 2000,
        deadline: 200,
    });
    items.push_back(crate::LockFundsItem {
        bounty_id: 3,
        depositor: depositor.clone(),
        amount: 2000,
        deadline: 300,
    });

    // Execute batch lock
    let locked_count = client.batch_lock_funds(&items);
    assert_eq!(locked_count, 3);

    // Verify all bounties are locked
    let escrow1 = client.get_escrow_info(&1);
    let escrow2 = client.get_escrow_info(&2);
    let escrow3 = client.get_escrow_info(&3);

    assert_eq!(escrow1.amount, 1000);
    assert_eq!(escrow2.amount, 2000);
    assert_eq!(escrow3.amount, 2000);
}

#[test]
fn test_batch_release_funds() {
    let (env, client, _contract_id) = create_test_env();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor1 = Address::generate(&env);
    let contributor2 = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let (token, _token_client, token_admin_client) = create_token_contract(&env, &token_admin);

    client.init(&admin, &token);

    // Lock funds for multiple bounties
    let amount1 = 1000i128;
    let amount2 = 2000i128;
    token_admin_client.mint(&depositor, &(amount1 + amount2));

    client.lock_funds(&depositor, &1, &amount1, &100);
    client.lock_funds(&depositor, &2, &amount2, &200);

    // Create batch release items
    let mut items = vec![&env];
    items.push_back(crate::ReleaseFundsItem {
        bounty_id: 1,
        contributor: contributor1.clone(),
    });
    items.push_back(crate::ReleaseFundsItem {
        bounty_id: 2,
        contributor: contributor2.clone(),
    });

    // Execute batch release
    let released_count = client.batch_release_funds(&items);
    assert_eq!(released_count, 2);

    // Verify funds were released
    let escrow1 = client.get_escrow_info(&1);
    let escrow2 = client.get_escrow_info(&2);

    assert_eq!(escrow1.status, crate::EscrowStatus::Released);
    assert_eq!(escrow2.status, crate::EscrowStatus::Released);
}

// ============================================================================
// Integration Tests: Error Propagation
// ============================================================================

#[test]
#[should_panic(expected = "Error(Contract, #11)")]
fn test_batch_lock_duplicate_bounty_id() {
    let (env, client, _contract_id) = create_test_env();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let (token, _token_client, token_admin_client) = create_token_contract(&env, &token_admin);

    client.init(&admin, &token);
    token_admin_client.mint(&depositor, &5000);

    // Create batch with duplicate bounty IDs
    let mut items = vec![&env];
    items.push_back(crate::LockFundsItem {
        bounty_id: 1,
        depositor: depositor.clone(),
        amount: 1000,
        deadline: 100,
    });
    items.push_back(crate::LockFundsItem {
        bounty_id: 1, // Duplicate!
        depositor: depositor.clone(),
        amount: 2000,
        deadline: 200,
    });

    client.batch_lock_funds(&items);
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")]
fn test_batch_lock_existing_bounty() {
    let (env, client, _contract_id) = create_test_env();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let (token, _token_client, token_admin_client) = create_token_contract(&env, &token_admin);

    client.init(&admin, &token);
    token_admin_client.mint(&depositor, &5000);

    // Lock a bounty first
    client.lock_funds(&depositor, &1, &1000, &100);

    // Try to batch lock the same bounty
    let mut items = vec![&env];
    items.push_back(crate::LockFundsItem {
        bounty_id: 1, // Already exists!
        depositor: depositor.clone(),
        amount: 2000,
        deadline: 200,
    });

    client.batch_lock_funds(&items);
}

// ============================================================================
// Integration Tests: Event Emission
// ============================================================================

#[test]
fn test_batch_lock_event_emission() {
    let (env, client, _contract_id) = create_test_env();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let (token, _token_client, token_admin_client) = create_token_contract(&env, &token_admin);

    client.init(&admin, &token);
    token_admin_client.mint(&depositor, &5000);

    let initial_event_count = env.events().all().len();

    // Create batch lock items
    let mut items = vec![&env];
    items.push_back(crate::LockFundsItem {
        bounty_id: 1,
        depositor: depositor.clone(),
        amount: 1000,
        deadline: 100,
    });
    items.push_back(crate::LockFundsItem {
        bounty_id: 2,
        depositor: depositor.clone(),
        amount: 2000,
        deadline: 200,
    });

    client.batch_lock_funds(&items);

    // Verify events were emitted (individual + batch events)
    let events = env.events().all();
    assert!(events.len() > initial_event_count);
}

#[test]
fn test_batch_release_event_emission() {
    let (env, client, _contract_id) = create_test_env();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor1 = Address::generate(&env);
    let contributor2 = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let (token, _token_client, token_admin_client) = create_token_contract(&env, &token_admin);

    client.init(&admin, &token);
    token_admin_client.mint(&depositor, &5000);

    // Lock funds
    client.lock_funds(&depositor, &1, &1000, &100);
    client.lock_funds(&depositor, &2, &2000, &200);

    let initial_event_count = env.events().all().len();

    // Create batch release items
    let mut items = vec![&env];
    items.push_back(crate::ReleaseFundsItem {
        bounty_id: 1,
        contributor: contributor1.clone(),
    });
    items.push_back(crate::ReleaseFundsItem {
        bounty_id: 2,
        contributor: contributor2.clone(),
    });

    client.batch_release_funds(&items);

    // Verify events were emitted
    let events = env.events().all();
    assert!(events.len() > initial_event_count);
}

// ============================================================================
// Integration Tests: Complete Workflow
// ============================================================================

#[test]
fn test_complete_bounty_workflow_lock_release() {
    let (env, client, _contract_id) = create_test_env();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let (token, token_client, token_admin_client) = create_token_contract(&env, &token_admin);

    // 1. Initialize contract
    client.init(&admin, &token);

    // 2. Mint tokens to depositor
    let amount = 5000i128;
    token_admin_client.mint(&depositor, &amount);

    // 3. Lock funds
    let bounty_id = 1u64;
    let deadline = 1000u64;
    client.lock_funds(&depositor, &bounty_id, &amount, &deadline);

    // 4. Verify funds locked
    let escrow = client.get_escrow_info(&bounty_id);
    assert_eq!(escrow.amount, amount);
    assert_eq!(escrow.status, crate::EscrowStatus::Locked);

    // 5. Verify contract balance
    let contract_balance = client.get_balance();
    assert_eq!(contract_balance, amount);

    // 6. Release funds to contributor
    client.release_funds(&bounty_id, &contributor);

    // 7. Verify funds released
    let escrow_after = client.get_escrow_info(&bounty_id);
    assert_eq!(escrow_after.status, crate::EscrowStatus::Released);

    // 8. Verify contributor received funds
    let contributor_balance = token_client.balance(&contributor);
    assert_eq!(contributor_balance, amount);
}

#[test]
fn test_complete_bounty_workflow_lock_refund() {
    let (env, client, _contract_id) = create_test_env();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let (token, token_client, token_admin_client) = create_token_contract(&env, &token_admin);

    client.init(&admin, &token);

    let amount = 5000i128;
    token_admin_client.mint(&depositor, &amount);

    let bounty_id = 1u64;
    // Use a future deadline, then advance the ledger timestamp past it
    let current_time = env.ledger().timestamp();
    let deadline = current_time + 1_000;
    client.lock_funds(&depositor, &bounty_id, &amount, &deadline);

    // Advance time past deadline so refund is eligible
    env.ledger().set_timestamp(deadline + 1);

    // Refund funds (deadline has already passed)
    client.refund(
        &bounty_id,
        &None::<i128>,
        &None::<Address>,
        &crate::RefundMode::Full,
    );

    // Verify funds refunded
    let escrow = client.get_escrow_info(&bounty_id);
    assert_eq!(escrow.status, crate::EscrowStatus::Refunded);

    // Verify depositor received refund
    let depositor_balance = token_client.balance(&depositor);
    assert_eq!(depositor_balance, amount);
}
