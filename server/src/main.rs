use gas_saver_eth::events::{GasEvent, TransactionRequest};
use gas_saver_eth::limiter::RateLimiter;
use gas_saver_eth::model::GasModel;
use gas_saver_eth::nonce::NonceManager;
use gas_saver_eth::scheduler::{Scheduler, SchedulerConfig};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{Level, info};
use tracing_subscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    let (gas_tx, gas_rx) = mpsc::channel(100);
    let (req_tx, req_rx) = mpsc::channel(100);
    let (decision_tx, mut decision_rx) = mpsc::channel(100);

    let model = Arc::new(GasModel::new(100));
    let nonce_manager = Arc::new(NonceManager::new());
    let limiter = Arc::new(RateLimiter::new(10, 20));

    let config = SchedulerConfig {
        target_base_fee: 50,
        max_priority_fee: 2,
        spike_threshold: 15.0,
        reprice_cooldown: tokio::time::Duration::from_millis(500),
    };

    let scheduler = Arc::new(Scheduler::new(
        config,
        model,
        nonce_manager,
        limiter,
        decision_tx,
    ));

    let _scheduler_handle = tokio::spawn(async move {
        scheduler.run(gas_rx, req_rx).await;
    });

    // Decision consumer
    tokio::spawn(async move {
        while let Some(decision) = decision_rx.recv().await {
            info!("CORE DECISION: {:?}", decision);
        }
    });

    info!("Starting GasSaver Simulation...");

    // 1. Initial stable state
    gas_tx
        .send(GasEvent::BaseFeeUpdate {
            base_fee: 50,
            timestamp: 1000,
        })
        .await?;
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // 2. Submit transaction
    let tx1 = TransactionRequest {
        id: 1,
        from: [0xAA; 20],
        to: [0xBB; 20],
        data: vec![],
        value: [0; 32],
        max_fee_per_gas: 100,
        max_priority_fee_per_gas: 2,
        deadline: None,
    };
    req_tx.send(tx1).await?;
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // 3. Gas rises - triggering reprice
    info!("Triggering gas rise for reprice...");
    gas_tx
        .send(GasEvent::BaseFeeUpdate {
            base_fee: 70,
            timestamp: 1100,
        })
        .await?;
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // 4. Gas Spike - triggering inclusion-first for new tx
    info!("Triggering gas spike...");
    for i in 0..10 {
        gas_tx
            .send(GasEvent::BaseFeeUpdate {
                base_fee: 70 + (i * 20),
                timestamp: 1200 + i,
            })
            .await?;
    }

    let tx2 = TransactionRequest {
        id: 2,
        from: [0xCC; 20],
        to: [0xDD; 20],
        data: vec![],
        value: [0; 32],
        max_fee_per_gas: 500,
        max_priority_fee_per_gas: 10,
        deadline: None,
    };
    req_tx.send(tx2).await?;

    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    info!("Simulation finished.");

    Ok(())
}
