use crate::events::{GasEvent, SchedulerDecision, TransactionRequest};
use crate::limiter::RateLimiter;
use crate::model::GasModel;
use crate::nonce::NonceManager;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing::{info, warn};

pub struct SchedulerConfig {
    pub target_base_fee: u64,
    pub max_priority_fee: u64,
    pub spike_threshold: f64,
    pub reprice_cooldown: Duration,
}

struct SubmittedTx {
    req: TransactionRequest,
    nonce: u64,
    last_gas_price: u64,
    last_action_at: Instant,
}

pub struct Scheduler {
    config: SchedulerConfig,
    model: Arc<GasModel>,
    nonce_manager: Arc<NonceManager>,
    limiter: Arc<RateLimiter>,
    decision_tx: mpsc::Sender<SchedulerDecision>,
}

impl Scheduler {
    pub fn new(
        config: SchedulerConfig,
        model: Arc<GasModel>,
        nonce_manager: Arc<NonceManager>,
        limiter: Arc<RateLimiter>,
        decision_tx: mpsc::Sender<SchedulerDecision>,
    ) -> Self {
        Self {
            config,
            model,
            nonce_manager,
            limiter,
            decision_tx,
        }
    }

    pub async fn run(
        self: Arc<Self>,
        mut gas_events: mpsc::Receiver<GasEvent>,
        mut tx_requests: mpsc::Receiver<TransactionRequest>,
    ) {
        let mut pending_txs: Vec<TransactionRequest> = Vec::new();
        let mut submitted_txs: HashMap<u64, SubmittedTx> = HashMap::new();

        loop {
            tokio::select! {
                Some(event) = gas_events.recv() => {
                    self.handle_gas_event(event, &mut pending_txs, &mut submitted_txs).await;
                }
                Some(req) = tx_requests.recv() => {
                    self.handle_tx_request(req, &mut pending_txs, &mut submitted_txs).await;
                }
                else => break,
            }
        }
    }

    async fn handle_gas_event(
        &self,
        event: GasEvent,
        pending_txs: &mut Vec<TransactionRequest>,
        submitted_txs: &mut HashMap<u64, SubmittedTx>,
    ) {
        match event {
            GasEvent::BaseFeeUpdate { base_fee, .. } | GasEvent::NewBlock { base_fee, .. } => {
                self.model.update(base_fee);
                self.re_evaluate_pending(pending_txs, submitted_txs).await;
            }
            GasEvent::TxConfirmed { tx_hash, .. } => {
                info!("Inclusion event for tx hash: {:?}", tx_hash);
                // Real-world: map hash to id and remove from submitted_txs
            }
            _ => {}
        }
    }

    async fn handle_tx_request(
        &self,
        req: TransactionRequest,
        pending_txs: &mut Vec<TransactionRequest>,
        submitted_txs: &mut HashMap<u64, SubmittedTx>,
    ) {
        pending_txs.push(req);
        self.re_evaluate_pending(pending_txs, submitted_txs).await;
    }

    async fn re_evaluate_pending(
        &self,
        pending_txs: &mut Vec<TransactionRequest>,
        submitted_txs: &mut HashMap<u64, SubmittedTx>,
    ) {
        let current_fee = self.model.current_fee();
        let volatility = self.model.get_volatility();
        let trend = self.model.get_trend();
        let is_spike = volatility > self.config.spike_threshold;

        // 1. Repricing with cooldown
        for tx in submitted_txs.values_mut() {
            if tx.last_action_at.elapsed() < self.config.reprice_cooldown {
                continue;
            }

            let min_new_price = (tx.last_gas_price * 110) / 100;
            let desired_price = current_fee + tx.req.max_priority_fee_per_gas;

            if desired_price > min_new_price && desired_price <= tx.req.max_fee_per_gas {
                warn!(
                    "REPRICING: tx {} from {} to {} (volatility: {:.2})",
                    tx.req.id, tx.last_gas_price, desired_price, volatility
                );

                let decision = SchedulerDecision::Reprice {
                    tx_id: tx.req.id,
                    old_nonce: tx.nonce,
                    new_gas_price: desired_price,
                };
                let _ = self.decision_tx.send(decision).await;
                tx.last_gas_price = desired_price;
                tx.last_action_at = Instant::now();
            }
        }

        // 2. Pending submission
        pending_txs.sort_by_key(|t| t.id);
        let mut to_remove = Vec::new();

        for (idx, tx) in pending_txs.iter().enumerate() {
            if !self.limiter.check_and_consume() {
                break;
            }

            let decision = if is_spike {
                info!("DEGRADATION MODE: Inclusion-first for tx {}", tx.id);
                let gas_price = current_fee + tx.max_priority_fee_per_gas;
                let nonce = self.nonce_manager.next_nonce(&tx.from);
                Some(SchedulerDecision::Submit {
                    tx_id: tx.id,
                    nonce,
                    gas_price,
                })
            } else if current_fee <= tx.max_fee_per_gas {
                let gas_price = current_fee + tx.max_priority_fee_per_gas;
                let nonce = self.nonce_manager.next_nonce(&tx.from);
                Some(SchedulerDecision::Submit {
                    tx_id: tx.id,
                    nonce,
                    gas_price,
                })
            } else if trend < -1.0 {
                // Significant downward trend
                info!(
                    "FEE HIGH but trending down ({:.2}). Deferring tx {}",
                    trend, tx.id
                );
                None
            } else {
                None
            };

            if let Some(d) = decision {
                if let SchedulerDecision::Submit {
                    tx_id,
                    nonce,
                    gas_price,
                } = d
                {
                    submitted_txs.insert(
                        tx_id,
                        SubmittedTx {
                            req: tx.clone(),
                            nonce,
                            last_gas_price: gas_price,
                            last_action_at: Instant::now(),
                        },
                    );
                    let _ = self.decision_tx.send(d).await;
                }
                to_remove.push(idx);
            }
        }

        for &idx in to_remove.iter().rev() {
            pending_txs.remove(idx);
        }
    }
}
