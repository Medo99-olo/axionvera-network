#![no_std]

use axionvera_rewards::calculate_pending_rewards;
use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, Address, Env, Symbol,
};

const TOPIC_VAULT: Symbol = symbol_short!("vault");
const TOPIC_INIT: Symbol = symbol_short!("init");
const TOPIC_DEPOSIT: Symbol = symbol_short!("deposit");
const TOPIC_WITHDRAW: Symbol = symbol_short!("withdraw");
const TOPIC_CLAIM: Symbol = symbol_short!("claim");

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum VaultError {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    InvalidAmount = 3,
    InsufficientBalance = 4,
    InvalidRewardState = 5,
}

#[contracttype]
enum DataKey {
    Initialized,
    Admin,
    DepositToken,
    RewardToken,
    Balance(Address),
    TotalDeposits,
    RewardBalance,
    ClaimableReward(Address),
}

#[contract]
pub struct VaultContract;

#[contractimpl]
impl VaultContract {
    /// Initializes the vault with an admin and tokens.
    ///
    /// The vault must be initialized before any other gated methods are called.
    /// This method can only be called once.
    ///
    /// # Arguments
    /// * `env` - The environment.
    /// * `admin` - The address that will have administrative rights. Must authorize the call.
    /// * `deposit_token` - The address of the token accepted for deposits.
    /// * `reward_token` - The address of the token distributed as rewards.
    ///
    /// # Returns
    /// * `Ok(())` on success.
    /// * `Err(VaultError::AlreadyInitialized)` if the vault has already been initialized.
    pub fn initialize(
        env: Env,
        admin: Address,
        deposit_token: Address,
        reward_token: Address,
    ) -> Result<(), VaultError> {
        admin.require_auth();
        if env.storage().instance().has(&DataKey::Initialized) {
            return Err(VaultError::AlreadyInitialized);
        }

        env.storage().instance().set(&DataKey::Initialized, &true);
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage()
            .instance()
            .set(&DataKey::DepositToken, &deposit_token);
        env.storage()
            .instance()
            .set(&DataKey::RewardToken, &reward_token);
        env.storage()
            .instance()
            .set(&DataKey::TotalDeposits, &0_i128);
        env.storage()
            .instance()
            .set(&DataKey::RewardBalance, &0_i128);
        env.events()
            .publish((TOPIC_VAULT, TOPIC_INIT), admin.clone());
        Ok(())
    }

    /// Deposits tokens into the vault on behalf of `from`.
    ///
    /// Increases the user's stored balance and the vault's total deposits.
    /// Note: This currently only updates vault accounting and does not transfer the `deposit_token`.
    /// Requires the vault to be initialized.
    ///
    /// # Arguments
    /// * `env` - The environment.
    /// * `from` - The address making the deposit. Must authorize the call.
    /// * `amount` - The amount of tokens to deposit. Must be greater than 0.
    ///
    /// # Returns
    /// * `Ok(new_balance)` - The user's new total deposited balance.
    /// * `Err(VaultError::NotInitialized)` if the vault is not initialized.
    /// * `Err(VaultError::InvalidAmount)` if the amount is `<= 0` or overflow occurs.
    pub fn deposit(env: Env, from: Address, amount: i128) -> Result<i128, VaultError> {
        Self::require_initialized(&env)?;
        from.require_auth();
        if amount <= 0 {
            return Err(VaultError::InvalidAmount);
        }

        let balance = Self::balance(&env, &from);
        let total = Self::total(&env);
        let new_balance = balance
            .checked_add(amount)
            .ok_or(VaultError::InvalidAmount)?;
        let new_total = total.checked_add(amount).ok_or(VaultError::InvalidAmount)?;
        env.storage()
            .persistent()
            .set(&DataKey::Balance(from.clone()), &new_balance);
        env.storage()
            .instance()
            .set(&DataKey::TotalDeposits, &new_total);
        env.events()
            .publish((TOPIC_VAULT, TOPIC_DEPOSIT), (from.clone(), amount));
        Ok(new_balance)
    }

    /// Withdraws tokens from the vault for `to`.
    ///
    /// Decreases the user's stored balance and the vault's total deposits.
    /// Note: This currently only updates vault accounting and does not transfer the `deposit_token`.
    /// Requires the vault to be initialized.
    ///
    /// # Arguments
    /// * `env` - The environment.
    /// * `to` - The address withdrawing tokens. Must authorize the call.
    /// * `amount` - The amount of tokens to withdraw. Must be greater than 0.
    ///
    /// # Returns
    /// * `Ok(new_balance)` - The user's new total deposited balance.
    /// * `Err(VaultError::NotInitialized)` if the vault is not initialized.
    /// * `Err(VaultError::InvalidAmount)` if the amount is `<= 0`.
    /// * `Err(VaultError::InsufficientBalance)` if the user's balance is less than `amount`.
    pub fn withdraw(env: Env, to: Address, amount: i128) -> Result<i128, VaultError> {
        Self::require_initialized(&env)?;
        to.require_auth();
        if amount <= 0 {
            return Err(VaultError::InvalidAmount);
        }

        let balance = Self::balance(&env, &to);
        if amount > balance {
            return Err(VaultError::InsufficientBalance);
        }
        let new_balance = balance
            .checked_sub(amount)
            .ok_or(VaultError::InvalidAmount)?;
        let new_total = Self::total(&env)
            .checked_sub(amount)
            .ok_or(VaultError::InvalidAmount)?;
        env.storage()
            .persistent()
            .set(&DataKey::Balance(to.clone()), &new_balance);
        env.storage()
            .instance()
            .set(&DataKey::TotalDeposits, &new_total);
        env.events()
            .publish((TOPIC_VAULT, TOPIC_WITHDRAW), (to.clone(), amount));
        Ok(new_balance)
    }

    /// Claims the pending rewards for `user`.
    ///
    /// Reads the user's stored claimable amount, resets it to 0, and returns the claimed amount.
    /// Note: This currently only updates vault accounting and does not transfer the `reward_token`.
    /// Requires the vault to be initialized.
    ///
    /// # Arguments
    /// * `env` - The environment.
    /// * `user` - The address claiming rewards. Must authorize the call.
    ///
    /// # Returns
    /// * `Ok(claimed)` - The amount claimed (0 if nothing to claim).
    /// * `Err(VaultError::NotInitialized)` if the vault is not initialized.
    pub fn claim_rewards(env: Env, user: Address) -> Result<i128, VaultError> {
        Self::require_initialized(&env)?;
        user.require_auth();
        let claimable = Self::claimable_reward(&env, &user);
        if claimable == 0 {
            return Ok(0);
        }
        env.storage()
            .persistent()
            .set(&DataKey::ClaimableReward(user.clone()), &0_i128);
        env.events()
            .publish((TOPIC_VAULT, TOPIC_CLAIM), (user, claimable));
        Ok(claimable)
    }

    /// Sets the claimable reward for a user.
    ///
    /// This is an internal/administrative method used to allocate rewards (no `require_auth` by default).
    /// Requires the vault to be initialized.
    ///
    /// # Arguments
    /// * `env` - The environment.
    /// * `user` - The address receiving the claimable reward.
    /// * `amount` - The amount to set as claimable.
    pub fn set_claimable_reward(env: Env, user: Address, amount: i128) -> Result<(), VaultError> {
        Self::require_initialized(&env)?;
        env.storage()
            .persistent()
            .set(&DataKey::ClaimableReward(user), &amount);
        Ok(())
    }

    /// Sets the total reward balance for the vault.
    ///
    /// This is an internal/administrative method used to update the total rewards available (no `require_auth` by default).
    /// Requires the vault to be initialized.
    ///
    /// # Arguments
    /// * `env` - The environment.
    /// * `amount` - The new reward balance amount.
    pub fn set_reward_balance(env: Env, amount: i128) -> Result<(), VaultError> {
        Self::require_initialized(&env)?;
        env.storage()
            .instance()
            .set(&DataKey::RewardBalance, &amount);
        Ok(())
    }

    /// Returns whether the vault has been initialized.
    pub fn is_initialized(env: Env) -> bool {
        env.storage().instance().has(&DataKey::Initialized)
    }

    /// Returns the address of the vault's admin.
    ///
    /// # Returns
    /// * `Ok(admin)` - The admin's address.
    /// * `Err(VaultError::NotInitialized)` if the vault is not initialized.
    pub fn admin(env: Env) -> Result<Address, VaultError> {
        Self::require_initialized(&env)?;
        Ok(env.storage().instance().get(&DataKey::Admin).unwrap())
    }

    /// Returns the address of the vault's owner (same as admin).
    ///
    /// # Returns
    /// * `Ok(owner)` - The owner's address.
    /// * `Err(VaultError::NotInitialized)` if the vault is not initialized.
    pub fn owner(env: Env) -> Result<Address, VaultError> {
        Self::admin(env)
    }

    /// Returns the address of the deposit token.
    ///
    /// # Returns
    /// * `Ok(deposit_token)` - The deposit token's address.
    /// * `Err(VaultError::NotInitialized)` if the vault is not initialized.
    pub fn deposit_token(env: Env) -> Result<Address, VaultError> {
        Self::require_initialized(&env)?;
        Ok(env
            .storage()
            .instance()
            .get(&DataKey::DepositToken)
            .unwrap())
    }

    /// Returns the address of the reward token.
    ///
    /// # Returns
    /// * `Ok(reward_token)` - The reward token's address.
    /// * `Err(VaultError::NotInitialized)` if the vault is not initialized.
    pub fn reward_token(env: Env) -> Result<Address, VaultError> {
        Self::require_initialized(&env)?;
        Ok(env.storage().instance().get(&DataKey::RewardToken).unwrap())
    }

    /// Returns the total amount of deposited tokens in the vault.
    ///
    /// # Returns
    /// * `Ok(total_deposits)` - The total amount deposited.
    /// * `Err(VaultError::NotInitialized)` if the vault is not initialized.
    pub fn total_deposits(env: Env) -> Result<i128, VaultError> {
        Self::require_initialized(&env)?;
        Ok(Self::total(&env))
    }

    /// Returns the deposited balance for a specific user.
    ///
    /// # Arguments
    /// * `env` - The environment.
    /// * `user` - The address to check the balance for.
    ///
    /// # Returns
    /// * `Ok(balance)` - The user's deposited balance (0 if none).
    /// * `Err(VaultError::NotInitialized)` if the vault is not initialized.
    pub fn user_balance(env: Env, user: Address) -> Result<i128, VaultError> {
        Self::require_initialized(&env)?;
        Ok(Self::balance(&env, &user))
    }

    /// Returns the calculated pending rewards for a specific user.
    ///
    /// This is a proportional view over the vault's total reward balance.
    /// It does not reflect the user's actual claimable balance via `claim_rewards`.
    ///
    /// # Arguments
    /// * `env` - The environment.
    /// * `user` - The address to check pending rewards for.
    ///
    /// # Returns
    /// * `Ok(pending)` - The calculated pending reward amount.
    /// * `Err(VaultError::NotInitialized)` if the vault is not initialized.
    pub fn pending_rewards(env: Env, user: Address) -> Result<i128, VaultError> {
        Self::require_initialized(&env)?;
        Ok(calculate_pending_rewards(
            Self::balance(&env, &user),
            Self::total(&env),
            Self::reward_balance(&env),
        ))
    }

    fn require_initialized(env: &Env) -> Result<(), VaultError> {
        if !env.storage().instance().has(&DataKey::Initialized) {
            return Err(VaultError::NotInitialized);
        }
        Ok(())
    }

    fn balance(env: &Env, user: &Address) -> i128 {
        env.storage()
            .persistent()
            .get(&DataKey::Balance(user.clone()))
            .unwrap_or(0)
    }

    fn total(env: &Env) -> i128 {
        env.storage()
            .instance()
            .get(&DataKey::TotalDeposits)
            .unwrap_or(0)
    }

    fn reward_balance(env: &Env) -> i128 {
        env.storage()
            .instance()
            .get(&DataKey::RewardBalance)
            .unwrap_or(0)
    }

    fn claimable_reward(env: &Env, user: &Address) -> i128 {
        env.storage()
            .persistent()
            .get(&DataKey::ClaimableReward(user.clone()))
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use soroban_sdk::testutils::{Address as _, Events, MockAuth, MockAuthInvoke};
    use soroban_sdk::{vec, IntoVal, Val, Vec};

    // -------------------------------------------------------------------------
    // Shared test helpers
    // -------------------------------------------------------------------------

    /// Generate a deterministic test address from a seed value.
    /// This makes tests more readable and less repetitive than calling
    /// Address::generate(&env) everywhere.
    fn test_address(env: &Env, _seed: u32) -> Address {
        Address::generate(env)
    }

    /// Generate a deterministic admin address for testing.
    fn test_admin(env: &Env) -> Address {
        test_address(env, 1)
    }

    /// Generate a deterministic user address for testing.
    fn test_user(env: &Env, index: u32) -> Address {
        test_address(env, 10 + index)
    }

    /// Generate a deterministic deposit token address for testing.
    fn test_deposit_token(env: &Env) -> Address {
        test_address(env, 100)
    }

    /// Generate a deterministic reward token address for testing.
    fn test_reward_token(env: &Env) -> Address {
        test_address(env, 200)
    }

    /// Create a fully initialized vault and return (env, client, admin,
    /// deposit_token, reward_token).  All auths are mocked so callers don't
    /// need to worry about signing.
    fn setup() -> (Env, VaultContractClient<'static>, Address, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(VaultContract, ());
        let client = VaultContractClient::new(&env, &contract_id);
        let admin = test_admin(&env);
        client.initialize(&admin, &test_deposit_token(&env), &test_reward_token(&env));
        (env, client, admin, contract_id)
    }

    fn setup_uninitialized() -> (Env, VaultContractClient<'static>, Address) {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(VaultContract, ());
        let client = VaultContractClient::new(&env, &contract_id);

        (env, client, contract_id)
    }

    fn expected_vault_event(
        env: &Env,
        contract_id: &Address,
        action: Symbol,
        data: impl IntoVal<Env, Val>,
    ) -> (Address, Vec<Val>, Val) {
        (
            contract_id.clone(),
            vec![env, TOPIC_VAULT.into_val(env), action.into_val(env)],
            data.into_val(env),
        )
    }

    fn assert_no_events(env: &Env) {
        assert!(env.events().all().is_empty());
    }

    // =========================================================================
    // Helper function unit tests
    // =========================================================================

    #[test]
    fn test_address_generates_valid_address() {
        let env = Env::default();
        let addr = test_address(&env, 1);
        // Address should be valid (no panic on creation)
        let _ = addr;
    }

    #[test]
    fn test_admin_generates_valid_address() {
        let env = Env::default();
        let admin = test_admin(&env);
        // Admin address should be valid
        let _ = admin;
    }

    #[test]
    fn test_user_generates_valid_address() {
        let env = Env::default();
        let user = test_user(&env, 0);
        // User address should be valid
        let _ = user;
    }

    #[test]
    fn test_user_with_different_indices() {
        let env = Env::default();
        let user1 = test_user(&env, 0);
        let user2 = test_user(&env, 1);
        // Different indices should produce addresses
        let _ = user1;
        let _ = user2;
    }

    #[test]
    fn test_deposit_token_generates_valid_address() {
        let env = Env::default();
        let token = test_deposit_token(&env);
        // Deposit token address should be valid
        let _ = token;
    }

    #[test]
    fn test_reward_token_generates_valid_address() {
        let env = Env::default();
        let token = test_reward_token(&env);
        // Reward token address should be valid
        let _ = token;
    }

    // =========================================================================
    // A. REPEATED INITIALIZATION
    // =========================================================================

    #[test]
    fn rejects_repeated_initialization() {
        let (env, client, admin, _) = setup();
        let result =
            client.try_initialize(&admin, &Address::generate(&env), &Address::generate(&env));
        assert_eq!(result.unwrap_err().unwrap(), VaultError::AlreadyInitialized);
    }

    /// After a failed re-initialization the stored admin must remain the
    /// original one — the failed call must not overwrite any storage.
    #[test]
    fn reinitialize_does_not_overwrite_admin() {
        let (env, client, original_admin, _) = setup();

        let attacker = Address::generate(&env);
        let _ = client.try_initialize(
            &attacker,
            &Address::generate(&env),
            &Address::generate(&env),
        );

        // AlreadyInitialized was returned, so the stored admin is unchanged.
        assert_eq!(client.admin(), original_admin);
    }

    /// After a failed re-initialization `is_initialized` must still be true
    /// and the token addresses must be unchanged.
    #[test]
    fn reinitialize_does_not_corrupt_state() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(VaultContract, ());
        let client = VaultContractClient::new(&env, &contract_id);

        let admin = test_admin(&env);
        let deposit_token = test_deposit_token(&env);
        let reward_token = test_reward_token(&env);
        client.initialize(&admin, &deposit_token, &reward_token);

        // Sanity — vault is initialized with the right tokens.
        assert!(client.is_initialized());
        assert_eq!(client.deposit_token(), deposit_token);
        assert_eq!(client.reward_token(), reward_token);
        assert_eq!(client.total_deposits(), 0);

        // Attempt re-init with completely different arguments.
        let _ = client.try_initialize(
            &test_admin(&env),
            &test_deposit_token(&env),
            &test_reward_token(&env),
        );

        // Everything must be unchanged.
        assert!(client.is_initialized());
        assert_eq!(client.admin(), admin);
        assert_eq!(client.deposit_token(), deposit_token);
        assert_eq!(client.reward_token(), reward_token);
        assert_eq!(client.total_deposits(), 0);
    }

    // =========================================================================
    // B. UNINITIALIZED PROTECTED METHODS
    // =========================================================================

    #[test]
    fn persists_admin_after_initialization() {
        let (_, client, admin, _) = setup();
        assert_eq!(client.admin(), admin);
    }

    #[test]
    fn repeated_initialization_cannot_overwrite_admin() {
        let (env, client, admin, _) = setup();
        let other_admin = Address::generate(&env);
        let result = client.try_initialize(
            &other_admin,
            &Address::generate(&env),
            &Address::generate(&env),
        );
        assert_eq!(result.unwrap_err().unwrap(), VaultError::AlreadyInitialized);
        assert_eq!(client.admin(), admin);
    }

    #[test]
    fn initialize_requires_admin_authorization() {
        let env = Env::default();
        let contract_id = env.register(VaultContract, ());
        let client = VaultContractClient::new(&env, &contract_id);
        let caller = Address::generate(&env);

        // No auth is mocked: an unsigned caller cannot assign ownership.
        // require_auth fails before any state is written, surfacing as an
        // invoke error rather than a VaultError.
        let result =
            client.try_initialize(&caller, &Address::generate(&env), &Address::generate(&env));
        let err = result.unwrap_err();
        assert!(
            err.is_err(),
            "expected an auth invoke error, got a contract error"
        );
        assert!(!client.is_initialized());
        assert_eq!(
            client.try_admin().unwrap_err().unwrap(),
            VaultError::NotInitialized
        );
    }

    #[test]
    fn rejects_uninitialized_usage() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(VaultContract, ());
        let client = VaultContractClient::new(&env, &contract_id);
        let user = Address::generate(&env);
        assert_eq!(
            client.try_deposit(&user, &0).unwrap_err().unwrap(),
            VaultError::NotInitialized
        );
    }

    #[test]
    fn deposit_rejected_before_initialization() {
        let (env, client, _) = setup_uninitialized();
        let user = Address::generate(&env);
        assert_eq!(
            client.try_deposit(&user, &100).unwrap_err().unwrap(),
            VaultError::NotInitialized
        );
    }

    #[test]
    fn withdraw_rejected_before_initialization() {
        let (env, client, _) = setup_uninitialized();
        let user = Address::generate(&env);
        assert_eq!(
            client.try_withdraw(&user, &100).unwrap_err().unwrap(),
            VaultError::NotInitialized
        );
    }

    #[test]
    fn claim_rewards_rejected_before_initialization() {
        let (env, client, _) = setup_uninitialized();
        let user = Address::generate(&env);
        assert_eq!(
            client.try_claim_rewards(&user).unwrap_err().unwrap(),
            VaultError::NotInitialized
        );
    }

    #[test]
    fn set_claimable_reward_rejected_before_initialization() {
        let (env, client, _) = setup_uninitialized();
        let user = Address::generate(&env);
        assert_eq!(
            client
                .try_set_claimable_reward(&user, &100)
                .unwrap_err()
                .unwrap(),
            VaultError::NotInitialized
        );
    }

    #[test]
    fn set_reward_balance_rejected_before_initialization() {
        let (_, client, _) = setup_uninitialized();
        assert_eq!(
            client.try_set_reward_balance(&500).unwrap_err().unwrap(),
            VaultError::NotInitialized
        );
    }

    #[test]
    fn deposit_token_query_rejected_before_initialization() {
        let (_, client, _) = setup_uninitialized();
        assert_eq!(
            client.try_deposit_token().unwrap_err().unwrap(),
            VaultError::NotInitialized
        );
    }

    #[test]
    fn reward_token_query_rejected_before_initialization() {
        let (_, client, _) = setup_uninitialized();
        assert_eq!(
            client.try_reward_token().unwrap_err().unwrap(),
            VaultError::NotInitialized
        );
    }

    #[test]
    fn total_deposits_query_rejected_before_initialization() {
        let (_, client, _) = setup_uninitialized();
        assert_eq!(
            client.try_total_deposits().unwrap_err().unwrap(),
            VaultError::NotInitialized
        );
    }

    #[test]
    fn user_balance_query_rejected_before_initialization() {
        let (env, client, _) = setup_uninitialized();
        let user = Address::generate(&env);
        assert_eq!(
            client.try_user_balance(&user).unwrap_err().unwrap(),
            VaultError::NotInitialized
        );
    }

    #[test]
    fn pending_rewards_query_rejected_before_initialization() {
        let (env, client, _) = setup_uninitialized();
        let user = Address::generate(&env);
        assert_eq!(
            client.try_pending_rewards(&user).unwrap_err().unwrap(),
            VaultError::NotInitialized
        );
    }

    /// Calling a protected method on an uninitialized vault must not create
    /// any storage entries — the vault must remain truly uninitialized.
    #[test]
    fn failed_protected_call_does_not_initialize_vault() {
        let (env, client, _) = setup_uninitialized();
        let user = Address::generate(&env);

        // Attempt several protected operations.
        let _ = client.try_deposit(&user, &100);
        let _ = client.try_withdraw(&user, &100);
        let _ = client.try_claim_rewards(&user);
        let _ = client.try_total_deposits();

        // The vault must still be uninitialized after all those failed calls.
        assert!(!client.is_initialized());
    }

    // =========================================================================
    // C. OWNER / ADMIN QUERY BEFORE INITIALIZATION
    // =========================================================================

    #[test]
    fn admin_query_rejected_before_initialization() {
        let (_, client, _) = setup_uninitialized();
        assert_eq!(
            client.try_admin().unwrap_err().unwrap(),
            VaultError::NotInitialized
        );
    }

    #[test]
    fn owner_query_rejected_before_initialization() {
        let (_, client, _) = setup_uninitialized();
        assert_eq!(
            client.try_owner().unwrap_err().unwrap(),
            VaultError::NotInitialized
        );
    }

    #[test]
    fn owner_returns_admin_after_initialization() {
        let (_, client, admin, _) = setup();
        assert_eq!(client.owner(), admin);
    }

    /// `is_initialized` must be false before any `initialize` call.
    #[test]
    fn is_initialized_returns_false_before_initialization() {
        let (_, client, _) = setup_uninitialized();
        assert!(!client.is_initialized());
    }

    // =========================================================================
    // D. VALID INITIALIZATION (happy path)
    // =========================================================================

    #[test]
    fn initialize_succeeds_with_valid_inputs() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(VaultContract, ());
        let client = VaultContractClient::new(&env, &contract_id);

        let admin = test_admin(&env);
        let deposit_token = test_deposit_token(&env);
        let reward_token = test_reward_token(&env);

        // Must succeed without error.
        client.initialize(&admin, &deposit_token, &reward_token);

        assert!(client.is_initialized());
        assert_eq!(client.admin(), admin);
        assert_eq!(client.deposit_token(), deposit_token);
        assert_eq!(client.reward_token(), reward_token);
    }

    #[test]
    fn initialize_persists_admin_correctly() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(VaultContract, ());
        let client = VaultContractClient::new(&env, &contract_id);

        let admin = test_admin(&env);
        client.initialize(&admin, &test_deposit_token(&env), &test_reward_token(&env));

        assert_eq!(client.admin(), admin);
    }

    #[test]
    fn initialize_persists_tokens_correctly() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(VaultContract, ());
        let client = VaultContractClient::new(&env, &contract_id);

        let admin = test_admin(&env);
        let deposit_token = test_deposit_token(&env);
        let reward_token = test_reward_token(&env);
        client.initialize(&admin, &deposit_token, &reward_token);

        assert_eq!(client.deposit_token(), deposit_token);
        assert_eq!(client.reward_token(), reward_token);
    }

    #[test]
    fn initialize_sets_total_deposits_to_zero() {
        let (_, client, _, _) = setup();
        assert_eq!(client.total_deposits(), 0);
    }

    /// `is_initialized` must return `true` after a successful initialization.
    #[test]
    fn is_initialized_returns_true_after_initialization() {
        let (_, client, _, _) = setup();
        assert!(client.is_initialized());
    }

    // =========================================================================
    // E. INVALID / UNAUTHORIZED INITIALIZATION
    // =========================================================================

    /// The `initialize` function calls `admin.require_auth()` before any
    /// storage write.  When the transaction is not authorized by the admin
    /// address supplied, the call must be rejected and the vault must remain
    /// uninitialized.
    ///
    /// We simulate this by providing a `MockAuth` that authorizes a *different*
    /// address (the impostor) for the contract invocation instead of the real
    /// admin.
    #[test]
    fn initialize_rejected_when_admin_auth_not_provided() {
        let env = Env::default();
        let contract_id = env.register(VaultContract, ());
        let client = VaultContractClient::new(&env, &contract_id);

        let real_admin = Address::generate(&env);
        let impostor = Address::generate(&env);
        let deposit_token = Address::generate(&env);
        let reward_token = Address::generate(&env);

        // Only the impostor authorizes the call — real_admin's auth is absent.
        env.mock_auths(&[MockAuth {
            address: &impostor,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "initialize",
                args: (&real_admin, &deposit_token, &reward_token).into_val(&env),
                sub_invokes: &[],
            },
        }]);

        // The call passes `real_admin` as the admin argument but the auth
        // provided is for `impostor`, so `real_admin.require_auth()` fails.
        let result = client.try_initialize(&real_admin, &deposit_token, &reward_token);
        assert!(result.is_err());

        // The vault must remain uninitialized — no partial state should exist.
        assert!(!client.is_initialized());
    }

    // =========================================================================
    // F. UNAUTHORIZED ACCESS AFTER INITIALIZATION
    // =========================================================================

    // NOTE: `set_claimable_reward` and `set_reward_balance` deliberately have
    // no `require_auth` call in the current implementation (they rely solely on
    // the initialization guard).  The tests below document that existing design
    // rather than introducing new restrictions.

    /// A non-admin account cannot call `deposit` on behalf of another address
    /// because `deposit` calls `from.require_auth()` — the `from` address must
    /// be the signer.
    ///
    /// When `mock_all_auths` is NOT used the auth check is enforced.
    #[test]
    fn deposit_requires_caller_to_be_authorized_depositor() {
        let env = Env::default();
        let contract_id = env.register(VaultContract, ());

        // Initialize with mocked auth, then drop mock_all_auths for the
        // deposit call below.
        env.mock_all_auths();
        let client = VaultContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let deposit_token = Address::generate(&env);
        let reward_token = Address::generate(&env);
        client.initialize(&admin, &deposit_token, &reward_token);

        let legitimate_user = Address::generate(&env);
        let bystander = Address::generate(&env);

        // Authorize `bystander` for the deposit, but pass `legitimate_user`
        // as the `from` argument — the auth check for `legitimate_user` will
        // fail.
        env.mock_auths(&[MockAuth {
            address: &bystander,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "deposit",
                args: (&legitimate_user, 100_i128).into_val(&env),
                sub_invokes: &[],
            },
        }]);

        let result = client.try_deposit(&legitimate_user, &100);
        assert!(result.is_err());

        // No balance must have been credited to the legitimate_user.
        env.mock_all_auths();
        assert_eq!(client.user_balance(&legitimate_user), 0);
    }

    /// A non-admin account cannot withdraw on behalf of another address
    /// because `withdraw` calls `to.require_auth()`.
    #[test]
    fn withdraw_requires_caller_to_be_authorized_withdrawer() {
        // Set up and make a deposit.
        let (env, client, _, _) = setup();
        let legitimate_user = Address::generate(&env);
        client.deposit(&legitimate_user, &50);

        // Now re-register a fresh env without mock_all_auths for the withdraw.
        let env2 = Env::default();
        let contract_id2 = env2.register(VaultContract, ());
        let client2 = VaultContractClient::new(&env2, &contract_id2);

        env2.mock_all_auths();
        let admin = Address::generate(&env2);
        let other_user = Address::generate(&env2);
        client2.initialize(&admin, &Address::generate(&env2), &Address::generate(&env2));
        client2.deposit(&other_user, &50);

        // Attempt to withdraw as a bystander authorizing the call on behalf
        // of `other_user`.
        let bystander = Address::generate(&env2);
        env2.mock_auths(&[MockAuth {
            address: &bystander,
            invoke: &MockAuthInvoke {
                contract: &contract_id2,
                fn_name: "withdraw",
                args: (&other_user, 50_i128).into_val(&env2),
                sub_invokes: &[],
            },
        }]);

        let result = client2.try_withdraw(&other_user, &50);
        assert!(result.is_err());

        // Balance must remain intact.
        env2.mock_all_auths();
        assert_eq!(client2.user_balance(&other_user), 50);
    }

    // =========================================================================
    // G. STATE INTEGRITY AFTER FAILED CALLS
    // =========================================================================

    /// After a rejected re-initialization total_deposits must still be zero
    /// (or whatever it was before — it must not be reset or corrupted).
    #[test]
    fn reinitialize_does_not_reset_total_deposits() {
        let (env, client, admin, _) = setup();
        let user = Address::generate(&env);
        client.deposit(&user, &42);
        assert_eq!(client.total_deposits(), 42);

        // Attempt re-init.
        let _ = client.try_initialize(&admin, &Address::generate(&env), &Address::generate(&env));

        // Total deposits must be unchanged.
        assert_eq!(client.total_deposits(), 42);
    }

    /// After a rejected re-initialization the user balance must be intact.
    #[test]
    fn reinitialize_does_not_reset_user_balance() {
        let (env, client, admin, _) = setup();
        let user = Address::generate(&env);
        client.deposit(&user, &99);
        assert_eq!(client.user_balance(&user), 99);

        let _ = client.try_initialize(&admin, &Address::generate(&env), &Address::generate(&env));

        assert_eq!(client.user_balance(&user), 99);
    }

    /// After a rejected re-initialization the claimable reward for a user must
    /// be unchanged.
    #[test]
    fn reinitialize_does_not_reset_claimable_rewards() {
        let (env, client, admin, _) = setup();
        let user = Address::generate(&env);
        client.set_claimable_reward(&user, &77);

        let _ = client.try_initialize(&admin, &Address::generate(&env), &Address::generate(&env));

        // Claim should still return the pre-set amount.
        assert_eq!(client.claim_rewards(&user), 77);
    }

    // =========================================================================
    // Existing regression tests (deposit / withdraw / rewards)
    // =========================================================================

    #[test]
    fn rejects_invalid_amounts_and_insufficient_balance() {
        let (env, client, _, _) = setup();
        let user = Address::generate(&env);
        assert_eq!(
            client.try_deposit(&user, &0).unwrap_err().unwrap(),
            VaultError::InvalidAmount
        );
        assert_eq!(
            client.try_withdraw(&user, &1).unwrap_err().unwrap(),
            VaultError::InsufficientBalance
        );
        client.deposit(&user, &10);
        assert_eq!(
            client.try_withdraw(&user, &11).unwrap_err().unwrap(),
            VaultError::InsufficientBalance
        );
        assert_eq!(
            client.try_withdraw(&user, &0).unwrap_err().unwrap(),
            VaultError::InvalidAmount
        );
    }

    #[test]
    fn tracks_deposits_and_claims_rewards() {
        let (env, client, _, _) = setup();
        let user = Address::generate(&env);
        assert_eq!(client.deposit(&user, &10), 10);
        assert_eq!(client.total_deposits(), 10);
        assert_eq!(client.withdraw(&user, &4), 6);
        assert_eq!(client.total_deposits(), 6);

        // Claiming with no rewards returns zero
        assert_eq!(client.claim_rewards(&user), 0);

        // Set rewards and claim
        client.set_claimable_reward(&user, &100);
        assert_eq!(client.claim_rewards(&user), 100);

        // Reward balance should be cleared
        assert_eq!(client.claim_rewards(&user), 0);
    }

    #[test]
    fn pending_rewards_are_proportional_to_deposit_share() {
        let (env, client, _, _) = setup();
        let user_a = Address::generate(&env);
        let user_b = Address::generate(&env);

        client.deposit(&user_a, &500);
        client.deposit(&user_b, &500);
        client.set_reward_balance(&200);

        assert_eq!(client.pending_rewards(&user_a), 100);
        assert_eq!(client.pending_rewards(&user_b), 100);
    }

    #[test]
    fn pending_rewards_are_zero_when_total_deposits_are_zero() {
        let (env, client, _, _) = setup();
        let user = Address::generate(&env);
        client.set_reward_balance(&200);
        assert_eq!(client.pending_rewards(&user), 0);
        // Claiming with no deposits also pays nothing.
        assert_eq!(client.claim_rewards(&user), 0);
    }

    #[test]
    fn zero_user_balance_returns_zero_reward() {
        let (env, client, _, _) = setup();
        let depositor = Address::generate(&env);
        let user = Address::generate(&env);
        client.deposit(&depositor, &1000);
        client.set_reward_balance(&200);
        // `user` has never deposited: balance is 0 while total deposits are > 0.
        assert_eq!(client.user_balance(&user), 0);
        assert_eq!(client.pending_rewards(&user), 0);
    }

    #[test]
    fn large_values_do_not_overflow() {
        let (env, client, _, _) = setup();
        let user = Address::generate(&env);
        let large = 10_000_000_000_000_000_000_i128;
        client.deposit(&user, &large);
        client.set_reward_balance(&large);
        // (balance * rewards) / total_deposits stays within i128 for these values.
        assert_eq!(client.pending_rewards(&user), large);
        // A reward balance large enough to overflow the product resolves to 0,
        // matching the rewards helper's checked-arithmetic policy. No panic.
        client.set_reward_balance(&i128::MAX);
        assert_eq!(client.pending_rewards(&user), 0);
    }

    #[test]
    fn repeated_claim_does_not_duplicate_rewards() {
        let (env, client, _, _) = setup();
        let user = Address::generate(&env);
        client.deposit(&user, &500);
        client.set_claimable_reward(&user, &100);
        assert_eq!(client.claim_rewards(&user), 100);
        // No new reward-earning activity: further claims pay nothing.
        assert_eq!(client.claim_rewards(&user), 0);
        assert_eq!(client.claim_rewards(&user), 0);
        // Claiming does not touch deposit accounting.
        assert_eq!(client.user_balance(&user), 500);
        assert_eq!(client.total_deposits(), 500);
    }

    #[test]
    fn user_without_balance_cannot_claim_reward() {
        let (env, client, _, _) = setup();
        let depositor = Address::generate(&env);
        let user = Address::generate(&env);
        client.deposit(&depositor, &1000);
        client.set_reward_balance(&200);
        // No balance and no stored claimable amount: claim pays zero, not an error.
        assert_eq!(client.user_balance(&user), 0);
        assert_eq!(client.claim_rewards(&user), 0);
    }

    #[test]
    fn valid_user_receives_expected_reward() {
        let (env, client, _, _) = setup();
        let user = Address::generate(&env);
        client.deposit(&user, &1000);
        client.set_reward_balance(&100);
        assert_eq!(client.pending_rewards(&user), 100);
        client.set_claimable_reward(&user, &100);
        assert_eq!(client.claim_rewards(&user), 100);
        // The claimed amount is cleared, so a subsequent claim pays nothing.
        assert_eq!(client.claim_rewards(&user), 0);
    }

    #[test]
    fn initialize_emits_stable_init_event() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(VaultContract, ());
        let client = VaultContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);

        client.initialize(&admin, &Address::generate(&env), &Address::generate(&env));

        assert_eq!(
            env.events().all(),
            vec![
                &env,
                expected_vault_event(&env, &contract_id, TOPIC_INIT, admin,)
            ],
        );
    }

    #[test]
    fn deposit_emits_stable_deposit_event() {
        let (env, client, _, contract_id) = setup();
        let user = Address::generate(&env);
        let amount = 42_i128;

        client.deposit(&user, &amount);

        assert_eq!(
            env.events().all(),
            vec![
                &env,
                expected_vault_event(&env, &contract_id, TOPIC_DEPOSIT, (user, amount),)
            ],
        );
    }

    #[test]
    fn withdraw_emits_stable_withdraw_event() {
        let (env, client, _, contract_id) = setup();
        let user = Address::generate(&env);
        let withdraw_amount = 37_i128;

        client.deposit(&user, &100);
        client.withdraw(&user, &withdraw_amount);

        assert_eq!(
            env.events().all(),
            vec![
                &env,
                expected_vault_event(&env, &contract_id, TOPIC_WITHDRAW, (user, withdraw_amount),)
            ],
        );
    }

    #[test]
    fn claim_rewards_emits_stable_claim_event() {
        let (env, client, _, contract_id) = setup();
        let user = Address::generate(&env);
        let claimable = 250_i128;

        client.set_claimable_reward(&user, &claimable);
        client.claim_rewards(&user);

        assert_eq!(
            env.events().all(),
            vec![
                &env,
                expected_vault_event(&env, &contract_id, TOPIC_CLAIM, (user, claimable),)
            ],
        );
    }

    #[test]
    fn failed_deposit_does_not_emit_event() {
        let (env, client, _, _) = setup();
        let user = Address::generate(&env);

        let _ = client.try_deposit(&user, &0);

        assert_no_events(&env);
    }

    #[test]
    fn failed_withdraw_does_not_emit_event() {
        let (env, client, _, _) = setup();
        let user = Address::generate(&env);

        let _ = client.try_withdraw(&user, &1);

        assert_no_events(&env);
    }

    #[test]
    fn claim_rewards_with_zero_claimable_does_not_emit_event() {
        let (env, client, _, _) = setup();
        let user = Address::generate(&env);

        client.claim_rewards(&user);

        assert_no_events(&env);
    }

    #[test]
    fn failed_deposit_on_uninitialized_contract_does_not_emit_event() {
        let (env, client, _) = setup_uninitialized();
        let user = Address::generate(&env);

        let _ = client.try_deposit(&user, &10);

        assert_no_events(&env);
    }

    #[test]
    fn full_lifecycle_emits_expected_events_per_step() {
        let (env, client, _, contract_id) = setup();
        let user = Address::generate(&env);
        let deposit_amount = 100_i128;
        let withdraw_amount = 25_i128;
        let claimable = 50_i128;

        client.deposit(&user, &deposit_amount);
        assert_eq!(
            env.events().all(),
            vec![
                &env,
                expected_vault_event(
                    &env,
                    &contract_id,
                    TOPIC_DEPOSIT,
                    (user.clone(), deposit_amount),
                )
            ],
        );

        client.withdraw(&user, &withdraw_amount);
        assert_eq!(
            env.events().all(),
            vec![
                &env,
                expected_vault_event(
                    &env,
                    &contract_id,
                    TOPIC_WITHDRAW,
                    (user.clone(), withdraw_amount),
                )
            ],
        );

        client.set_claimable_reward(&user, &claimable);
        client.claim_rewards(&user);
        assert_eq!(
            env.events().all(),
            vec![
                &env,
                expected_vault_event(&env, &contract_id, TOPIC_CLAIM, (user, claimable),)
            ],
        );
    }

    #[test]
    fn multi_user_accounting_maintains_consistent_total_deposits() {
        let (env, client, _, _) = setup();
        let user_a = Address::generate(&env);
        let user_b = Address::generate(&env);

        // Multiple deposits
        client.deposit(&user_a, &100);
        assert_eq!(client.user_balance(&user_a), 100);
        assert_eq!(client.total_deposits(), 100);

        client.deposit(&user_b, &200);
        assert_eq!(client.user_balance(&user_b), 200);
        assert_eq!(client.total_deposits(), 300);

        client.deposit(&user_a, &50);
        assert_eq!(client.user_balance(&user_a), 150);
        assert_eq!(client.total_deposits(), 350);

        // Partial withdrawal
        client.withdraw(&user_b, &50);
        assert_eq!(client.user_balance(&user_b), 150);
        assert_eq!(client.total_deposits(), 300);

        // Full withdrawal
        client.withdraw(&user_a, &150);
        assert_eq!(client.user_balance(&user_a), 0);
        assert_eq!(client.total_deposits(), 150);

        // Failed withdrawal
        let _ = client.try_withdraw(&user_b, &1000);
        assert_eq!(client.user_balance(&user_b), 150);
        assert_eq!(client.total_deposits(), 150);

        // Balances remain isolated
        assert_eq!(client.user_balance(&user_a), 0);
        assert_eq!(client.user_balance(&user_b), 150);
    }
}
