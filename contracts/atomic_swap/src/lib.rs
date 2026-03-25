#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Address, BytesN, Env};

// ── Storage Keys ─────────────────────────────────────────────────────────────

#[contracttype]
pub enum DataKey {
    Swap(u64),
    NextId,
    /// Maps ip_id → swap_id for any swap currently in Pending or Accepted state.
    /// Cleared when a swap reaches Completed or Cancelled.
    ActiveSwap(u64),
}

// ── Types ─────────────────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone, PartialEq, Eq)]
pub enum SwapStatus {
    Pending,
    Accepted,
    Completed,
    Cancelled,
}

#[contracttype]
#[derive(Clone)]
pub struct SwapRecord {
    pub ip_id: u64,
    pub seller: Address,
    pub buyer: Address,
    pub price: i128,
    pub status: SwapStatus,
}

// ── Contract ──────────────────────────────────────────────────────────────────

#[contract]
pub struct AtomicSwap;

#[contractimpl]
impl AtomicSwap {
    /// Seller initiates a patent sale. Returns the swap ID.
    /// Panics if an active (Pending or Accepted) swap already exists for this ip_id.
    pub fn initiate_swap(env: Env, ip_id: u64, price: i128, buyer: Address) -> u64 {
        // Guard: reject if an active swap already exists for this IP
        assert!(
            !env.storage().persistent().has(&DataKey::ActiveSwap(ip_id)),
            "active swap already exists for this ip_id"
        );

        let seller = env.current_contract_address(); // placeholder; real impl uses invoker
        let id: u64 = env.storage().instance().get(&DataKey::NextId).unwrap_or(0);

        let swap = SwapRecord {
            ip_id,
            seller,
            buyer,
            price,
            status: SwapStatus::Pending,
        };

        env.storage().persistent().set(&DataKey::Swap(id), &swap);
        env.storage().persistent().set(&DataKey::ActiveSwap(ip_id), &id);
        env.storage().instance().set(&DataKey::NextId, &(id + 1));
        id
    }

    /// Buyer accepts the swap and sends payment (payment handled by token contract in full impl).
    pub fn accept_swap(env: Env, swap_id: u64) {
        let mut swap: SwapRecord = env
            .storage()
            .persistent()
            .get(&DataKey::Swap(swap_id))
            .expect("swap not found");

        assert!(swap.status == SwapStatus::Pending, "swap not pending");
        swap.status = SwapStatus::Accepted;
        env.storage().persistent().set(&DataKey::Swap(swap_id), &swap);
    }

    /// Seller reveals the decryption key; payment releases.
    pub fn reveal_key(env: Env, swap_id: u64, _decryption_key: BytesN<32>) {
        let mut swap: SwapRecord = env
            .storage()
            .persistent()
            .get(&DataKey::Swap(swap_id))
            .expect("swap not found");

        assert!(swap.status == SwapStatus::Accepted, "swap not accepted");
        // Full impl: verify key against IP commitment, then transfer escrowed payment
        swap.status = SwapStatus::Completed;
        env.storage().persistent().set(&DataKey::Swap(swap_id), &swap);
        // Release the IP lock so a new swap can be created
        env.storage().persistent().remove(&DataKey::ActiveSwap(swap.ip_id));
    }

    /// Cancel a swap (invalid key or timeout).
    pub fn cancel_swap(env: Env, swap_id: u64) {
        let mut swap: SwapRecord = env
            .storage()
            .persistent()
            .get(&DataKey::Swap(swap_id))
            .expect("swap not found");

        assert!(
            swap.status == SwapStatus::Pending || swap.status == SwapStatus::Accepted,
            "swap already finalised"
        );
        swap.status = SwapStatus::Cancelled;
        env.storage().persistent().set(&DataKey::Swap(swap_id), &swap);
        // Release the IP lock so a new swap can be created
        env.storage().persistent().remove(&DataKey::ActiveSwap(swap.ip_id));
    }

    /// Read a swap record. Returns None if the swap_id does not exist.
    pub fn get_swap(env: Env, swap_id: u64) -> Option<SwapRecord> {
        env.storage().persistent().get(&DataKey::Swap(swap_id))
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::Env;

    #[test]
    fn get_swap_returns_none_for_nonexistent_id() {
        let env = Env::default();
        let contract_id = env.register(AtomicSwap, ());
        let client = AtomicSwapClient::new(&env, &contract_id);

        // No swaps have been created; any ID should return None
        let result = client.get_swap(&9999);
        assert!(result.is_none());
    }

    #[test]
    fn get_swap_returns_some_for_existing_swap() {
        let env = Env::default();
        let contract_id = env.register(AtomicSwap, ());
        let client = AtomicSwapClient::new(&env, &contract_id);

        let buyer = Address::generate(&env);
        let swap_id = client.initiate_swap(&1_u64, &100_i128, &buyer);

        let result = client.get_swap(&swap_id);
        assert!(result.is_some());
        let swap = result.unwrap();
        assert_eq!(swap.ip_id, 1_u64);
        assert_eq!(swap.price, 100_i128);
        assert_eq!(swap.status, SwapStatus::Pending);
    }

    /// A second initiate_swap for the same ip_id must be rejected while the first is active.
    #[test]
    fn duplicate_swap_rejected_while_active() {
        let env = Env::default();
        let contract_id = env.register(AtomicSwap, ());
        let client = AtomicSwapClient::new(&env, &contract_id);

        let buyer = Address::generate(&env);
        client.initiate_swap(&1_u64, &100_i128, &buyer);
        // Second call for the same ip_id must fail
        let result = client.try_initiate_swap(&1_u64, &200_i128, &buyer);
        assert!(result.is_err());
    }

    /// After a swap is cancelled the IP lock is released and a new swap can be created.
    #[test]
    fn new_swap_allowed_after_cancel() {
        let env = Env::default();
        let contract_id = env.register(AtomicSwap, ());
        let client = AtomicSwapClient::new(&env, &contract_id);

        let buyer = Address::generate(&env);
        let swap_id = client.initiate_swap(&2_u64, &100_i128, &buyer);
        client.cancel_swap(&swap_id);

        // Lock released — this must succeed
        let new_id = client.initiate_swap(&2_u64, &150_i128, &buyer);
        assert_ne!(new_id, swap_id);
    }

    /// After a swap completes the IP lock is released and a new swap can be created.
    #[test]
    fn new_swap_allowed_after_complete() {
        let env = Env::default();
        let contract_id = env.register(AtomicSwap, ());
        let client = AtomicSwapClient::new(&env, &contract_id);

        let buyer = Address::generate(&env);
        let swap_id = client.initiate_swap(&3_u64, &100_i128, &buyer);
        client.accept_swap(&swap_id);

        let key = soroban_sdk::BytesN::from_array(&env, &[0u8; 32]);
        client.reveal_key(&swap_id, &key);

        // Lock released — this must succeed
        let new_id = client.initiate_swap(&3_u64, &150_i128, &buyer);
        assert_ne!(new_id, swap_id);
    }
}
