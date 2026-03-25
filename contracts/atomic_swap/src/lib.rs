#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Address, BytesN, Env};

// ── Storage Keys ─────────────────────────────────────────────────────────────

#[contracttype]
pub enum DataKey {
    Swap(u64),
    NextId,
}

// ── Types ─────────────────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone, PartialEq)]
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

// ── Events ────────────────────────────────────────────────────────────────────

/// Payload published when a swap is successfully cancelled.
/// Topic: `swp_cncld` (symbol_short, max 9 chars) — used by off-chain indexers.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct SwapCancelledEvent {
    pub swap_id: u64,
    pub canceller: Address,
}

// ── Contract ──────────────────────────────────────────────────────────────────

#[contract]
pub struct AtomicSwap;

#[contractimpl]
impl AtomicSwap {
    /// Seller initiates a patent sale. Returns the swap ID.
    pub fn initiate_swap(env: Env, ip_id: u64, price: i128, buyer: Address) -> u64 {
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
    }

    /// Cancel a swap (invalid key or timeout). Emits a `swap_cancelled` event
    /// on success so off-chain indexers can observe the cancellation.
    pub fn cancel_swap(env: Env, swap_id: u64, canceller: Address) {
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

        // Emit cancellation event — only reached on successful state transition.
        env.events().publish(
            (symbol_short!("swp_cncld"),),
            SwapCancelledEvent { swap_id, canceller },
        );
    }

    /// Read a swap record.
    pub fn get_swap(env: Env, swap_id: u64) -> SwapRecord {
        env.storage()
            .persistent()
            .get(&DataKey::Swap(swap_id))
            .expect("swap not found")
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{
        testutils::{Address as _, Events},
        vec, Env, IntoVal,
    };

    fn setup() -> (Env, Address, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, AtomicSwap);
        let canceller = Address::generate(&env);
        (env, contract_id, canceller)
    }

    fn make_swap(env: &Env, client: &AtomicSwapClient) -> u64 {
        let buyer = Address::generate(env);
        client.initiate_swap(&1u64, &1000_i128, &buyer)
    }

    #[test]
    fn test_cancel_pending_swap_emits_event() {
        let (env, contract_id, canceller) = setup();
        let client = AtomicSwapClient::new(&env, &contract_id);
        let swap_id = make_swap(&env, &client);

        client.cancel_swap(&swap_id, &canceller);

        // Confirm state transitioned
        assert_eq!(client.get_swap(&swap_id).status, SwapStatus::Cancelled);

        // Assert the event was emitted with the correct topic and payload
        let events = env.events().all();
        assert_eq!(events.len(), 1);
        let (_, topics, data) = events.get(0).unwrap();
        assert_eq!(topics, vec![&env, symbol_short!("swp_cncld").into_val(&env)]);
        let payload: SwapCancelledEvent = data.into_val(&env);
        assert_eq!(payload.swap_id, swap_id);
        assert_eq!(payload.canceller, canceller);
    }

    #[test]
    fn test_cancel_accepted_swap_emits_event() {
        let (env, contract_id, canceller) = setup();
        let client = AtomicSwapClient::new(&env, &contract_id);
        let swap_id = make_swap(&env, &client);

        client.accept_swap(&swap_id);
        client.cancel_swap(&swap_id, &canceller);

        assert_eq!(client.get_swap(&swap_id).status, SwapStatus::Cancelled);

        let events = env.events().all();
        assert_eq!(events.len(), 1);
        let (_, _, data) = events.get(0).unwrap();
        let payload: SwapCancelledEvent = data.into_val(&env);
        assert_eq!(payload.swap_id, swap_id);
        assert_eq!(payload.canceller, canceller);
    }

    #[test]
    #[should_panic(expected = "swap already finalised")]
    fn test_cancel_completed_swap_fails_no_event() {
        let (env, contract_id, canceller) = setup();
        let client = AtomicSwapClient::new(&env, &contract_id);
        let swap_id = make_swap(&env, &client);

        client.accept_swap(&swap_id);
        client.reveal_key(&swap_id, &soroban_sdk::testutils::BytesN::random(&env));

        // This must panic — no event should be emitted
        client.cancel_swap(&swap_id, &canceller);
    }

    /// Confirms no swap_cancelled event is emitted when the swap completes normally.
    /// A completed swap has no cancellation event — the events list stays empty.
    #[test]
    fn test_no_cancel_event_when_swap_completed_normally() {
        let (env, contract_id, _canceller) = setup();
        let client = AtomicSwapClient::new(&env, &contract_id);
        let swap_id = make_swap(&env, &client);

        client.accept_swap(&swap_id);
        client.reveal_key(&swap_id, &soroban_sdk::testutils::BytesN::random(&env));

        // Swap completed via reveal_key — no cancellation event should exist
        assert_eq!(env.events().all().len(), 0);
    }
}
