use alloy_primitives::{Address, B256, U256};
use borsh::{BorshDeserialize, BorshSerialize};

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq)]
pub enum GasEvent {
    BaseFeeUpdate {
        base_fee: u64,
        timestamp: u64,
    },
    MempoolTx {
        tx_hash: [u8; 32],
        max_fee: u64,
        max_priority_fee: u64,
        gas_limit: u64,
    },
    NewBlock {
        number: u64,
        base_fee: u64,
        gas_used: u64,
        gas_limit: u64,
    },
    TxConfirmed {
        tx_hash: [u8; 32],
        block_number: u64,
    },
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct TransactionRequest {
    pub id: u64,
    pub from: [u8; 20],
    pub to: [u8; 20],
    pub data: Vec<u8>,
    pub value: [u8; 32], // U256 as bytes
    pub max_fee_per_gas: u64,
    pub max_priority_fee_per_gas: u64,
    pub deadline: Option<u64>,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub enum SchedulerDecision {
    Submit {
        tx_id: u64,
        nonce: u64,
        gas_price: u64,
    },
    Defer {
        tx_id: u64,
        reason: String,
    },
    Reprice {
        tx_id: u64,
        old_nonce: u64,
        new_gas_price: u64,
    },
    Drop {
        tx_id: u64,
        reason: String,
    },
}
