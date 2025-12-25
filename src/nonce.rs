use dashmap::DashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

pub struct NonceManager {
    /// Maps Address to its next expected nonce
    nonces: DashMap<[u8; 20], Arc<AtomicU64>>,
}

impl NonceManager {
    pub fn new() -> Self {
        Self {
            nonces: DashMap::new(),
        }
    }

    /// Allocates a nonce for a given address.
    /// If the address is unknown, it should ideally be initialized from the network first.
    /// For this implementation, we'll assume initialization happens separately.
    pub fn next_nonce(&self, address: &[u8; 20]) -> u64 {
        let entry = self
            .nonces
            .entry(*address)
            .or_insert_with(|| Arc::new(AtomicU64::new(0)));
        entry.fetch_add(1, Ordering::SeqCst)
    }

    /// Peek at the current nonce without incrementing
    pub fn peek_nonce(&self, address: &[u8; 20]) -> u64 {
        self.nonces
            .get(address)
            .map(|v| v.load(Ordering::SeqCst))
            .unwrap_or(0)
    }

    /// Update the nonce (e.g., if a transaction fails with "nonce too low" or on startup)
    pub fn update_nonce(&self, address: [u8; 20], new_nonce: u64) {
        let entry = self
            .nonces
            .entry(address)
            .or_insert_with(|| Arc::new(AtomicU64::new(new_nonce)));
        entry.store(new_nonce, Ordering::SeqCst);
    }
}
