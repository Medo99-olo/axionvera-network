// Integration tests for vault contract lifecycle flows
// These tests verify the full workflow from initialization through deposits, withdrawals, and rewards

use axionvera_vault_contract::VaultContract;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{Address, Env};

// -------------------------------------------------------------------------
// Lifecycle test helpers
// -------------------------------------------------------------------------

/// Represents the state of a user in the vault lifecycle
#[derive(Debug, Clone)]
struct UserState {
    address: Address,
    expected_balance: i128,
    expected_claimable: i128,
}

impl UserState {
    fn new(address: Address) -> Self {
        Self {
            address,
            expected_balance: 0,
            expected_claimable: 0,
        }
    }

    fn with_balance(mut self, balance: i128) -> Self {
        self.expected_balance = balance;
        self
    }

    fn with_claimable(mut self, claimable: i128) -> Self {
        self.expected_claimable = claimable;
        self
    }
}

/// Represents the expected vault state after lifecycle operations
#[derive(Debug, Clone)]
struct VaultState {
    expected_total_deposits: i128,
}

impl VaultState {
    fn new() -> Self {
        Self {
            expected_total_deposits: 0,
        }
    }

    fn with_total_deposits(mut self, total: i128) -> Self {
        self.expected_total_deposits = total;
        self
    }
}

/// Vault lifecycle test helper that manages setup and state verification
struct VaultLifecycle {
    env: Env,
    client: axionvera_vault_contract::VaultContractClient<'static>,
    _admin: Address,
    _deposit_token: Address,
    _reward_token: Address,
}

impl VaultLifecycle {
    /// Create a new initialized vault for lifecycle testing
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(VaultContract, ());
        let client = axionvera_vault_contract::VaultContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let deposit_token = Address::generate(&env);
        let reward_token = Address::generate(&env);

        client.initialize(&admin, &deposit_token, &reward_token);

        Self {
            env,
            client,
            _admin: admin,
            _deposit_token: deposit_token,
            _reward_token: reward_token,
        }
    }

    /// Verify that the vault matches the expected state
    fn verify_vault_state(&self, expected: &VaultState) {
        assert_eq!(
            self.client.total_deposits(),
            expected.expected_total_deposits,
            "Total deposits mismatch"
        );
    }

    /// Verify that a user matches the expected state
    fn verify_user_state(&self, user: &UserState) {
        assert_eq!(
            self.client.user_balance(&user.address),
            user.expected_balance,
            "User balance mismatch for address"
        );
        // Note: claimable_reward is not publicly exposed, so we track it in expected state
        // but cannot verify it directly against the contract
    }

    /// Perform a deposit and update expected state
    fn deposit(&self, user: &mut UserState, amount: i128, vault_state: &mut VaultState) {
        self.client.deposit(&user.address, &amount);
        user.expected_balance += amount;
        vault_state.expected_total_deposits += amount;
    }

    /// Perform a withdrawal and update expected state
    fn withdraw(&self, user: &mut UserState, amount: i128, vault_state: &mut VaultState) {
        self.client.withdraw(&user.address, &amount);
        user.expected_balance -= amount;
        vault_state.expected_total_deposits -= amount;
    }

    /// Set claimable reward for a user and update expected state
    fn set_claimable(&self, user: &mut UserState, amount: i128, _vault_state: &mut VaultState) {
        self.client.set_claimable_reward(&user.address, &amount);
        user.expected_claimable = amount;
    }

    /// Claim rewards for a user and update expected state
    fn claim_rewards(&self, user: &mut UserState, _vault_state: &mut VaultState) -> i128 {
        let claimed = self.client.claim_rewards(&user.address);
        user.expected_claimable = 0;
        claimed
    }
}

// -------------------------------------------------------------------------
// Unit tests for helper functions
// -------------------------------------------------------------------------

#[test]
fn user_state_new_creates_zero_state() {
    let env = Env::default();
    let address = Address::generate(&env);
    let state = UserState::new(address.clone());

    assert_eq!(state.expected_balance, 0);
    assert_eq!(state.expected_claimable, 0);
}

#[test]
fn user_state_with_balance_updates_balance() {
    let env = Env::default();
    let address = Address::generate(&env);
    let state = UserState::new(address).with_balance(100);

    assert_eq!(state.expected_balance, 100);
    assert_eq!(state.expected_claimable, 0);
}

#[test]
fn user_state_with_claimable_updates_claimable() {
    let env = Env::default();
    let address = Address::generate(&env);
    let state = UserState::new(address).with_claimable(50);

    assert_eq!(state.expected_balance, 0);
    assert_eq!(state.expected_claimable, 50);
}

#[test]
fn user_state_chaining_works() {
    let env = Env::default();
    let address = Address::generate(&env);
    let state = UserState::new(address).with_balance(100).with_claimable(50);

    assert_eq!(state.expected_balance, 100);
    assert_eq!(state.expected_claimable, 50);
}

#[test]
fn vault_state_new_creates_zero_state() {
    let state = VaultState::new();

    assert_eq!(state.expected_total_deposits, 0);
}

#[test]
fn vault_state_with_total_deposits_updates_total() {
    let state = VaultState::new().with_total_deposits(1000);

    assert_eq!(state.expected_total_deposits, 1000);
}

// -------------------------------------------------------------------------
// Lifecycle integration tests
// -------------------------------------------------------------------------

#[test]
fn full_lifecycle_initialize_deposit_withdraw_claim() {
    let lifecycle = VaultLifecycle::new();

    // Initial state
    let mut vault_state = VaultState::new();
    lifecycle.verify_vault_state(&vault_state);

    // Create users
    let user1 = Address::generate(&lifecycle.env);
    let mut user1_state = UserState::new(user1.clone());

    let user2 = Address::generate(&lifecycle.env);
    let mut user2_state = UserState::new(user2.clone());

    // Verify initial user balances are zero
    lifecycle.verify_user_state(&user1_state);
    lifecycle.verify_user_state(&user2_state);

    // User1 deposits
    lifecycle.deposit(&mut user1_state, 100, &mut vault_state);
    lifecycle.verify_user_state(&user1_state);
    lifecycle.verify_vault_state(&vault_state);

    // User2 deposits
    lifecycle.deposit(&mut user2_state, 200, &mut vault_state);
    lifecycle.verify_user_state(&user2_state);
    lifecycle.verify_vault_state(&vault_state);

    // Set claimable rewards for users
    lifecycle.set_claimable(&mut user1_state, 50, &mut vault_state);
    lifecycle.verify_user_state(&user1_state);
    lifecycle.verify_vault_state(&vault_state);

    lifecycle.set_claimable(&mut user2_state, 100, &mut vault_state);
    lifecycle.verify_user_state(&user2_state);
    lifecycle.verify_vault_state(&vault_state);

    // User1 claims rewards
    let claimed = lifecycle.claim_rewards(&mut user1_state, &mut vault_state);
    assert_eq!(claimed, 50);
    lifecycle.verify_user_state(&user1_state);
    lifecycle.verify_vault_state(&vault_state);

    // User1 withdraws
    lifecycle.withdraw(&mut user1_state, 30, &mut vault_state);
    lifecycle.verify_user_state(&user1_state);
    lifecycle.verify_vault_state(&vault_state);

    // User2 claims rewards
    let claimed = lifecycle.claim_rewards(&mut user2_state, &mut vault_state);
    assert_eq!(claimed, 100);
    lifecycle.verify_user_state(&user2_state);
    lifecycle.verify_vault_state(&vault_state);

    // Final state verification
    assert_eq!(user1_state.expected_balance, 70);
    assert_eq!(user2_state.expected_balance, 200);
    assert_eq!(vault_state.expected_total_deposits, 270);
}

#[test]
fn lifecycle_multiple_deposits_and_withdrawals() {
    let lifecycle = VaultLifecycle::new();

    let mut vault_state = VaultState::new();
    let user = Address::generate(&lifecycle.env);
    let mut user_state = UserState::new(user.clone());

    // Multiple deposits
    lifecycle.deposit(&mut user_state, 100, &mut vault_state);
    lifecycle.deposit(&mut user_state, 50, &mut vault_state);
    lifecycle.deposit(&mut user_state, 25, &mut vault_state);

    lifecycle.verify_user_state(&user_state);
    assert_eq!(user_state.expected_balance, 175);
    assert_eq!(vault_state.expected_total_deposits, 175);

    // Multiple withdrawals
    lifecycle.withdraw(&mut user_state, 30, &mut vault_state);
    lifecycle.withdraw(&mut user_state, 20, &mut vault_state);

    lifecycle.verify_user_state(&user_state);
    assert_eq!(user_state.expected_balance, 125);
    assert_eq!(vault_state.expected_total_deposits, 125);
}

#[test]
fn lifecycle_reward_claim_resets_claimable() {
    let lifecycle = VaultLifecycle::new();

    let mut vault_state = VaultState::new();
    let user = Address::generate(&lifecycle.env);
    let mut user_state = UserState::new(user.clone());

    // Deposit and set claimable
    lifecycle.deposit(&mut user_state, 100, &mut vault_state);
    lifecycle.set_claimable(&mut user_state, 75, &mut vault_state);

    lifecycle.verify_user_state(&user_state);
    assert_eq!(user_state.expected_claimable, 75);

    // Claim rewards
    lifecycle.claim_rewards(&mut user_state, &mut vault_state);

    // Verify claimable is reset to 0
    lifecycle.verify_user_state(&user_state);
    assert_eq!(user_state.expected_claimable, 0);

    // Claim again should return 0
    let claimed = lifecycle.claim_rewards(&mut user_state, &mut vault_state);
    assert_eq!(claimed, 0);
}
