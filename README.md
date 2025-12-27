# GasSaver

**GasSaver** is a high-performance, event-driven, gas-aware transaction scheduling system implemented in Rust. It is designed to optimize Ethereum transaction submission by reacting to real-time network conditions, ensuring cost-efficiency during normal operations and reliability during extreme gas spikes.

## 🚀 Features

- **Event-Driven Architecture**: Fully asynchronous design using `Tokio`. It consumes mempool event streams (base fee updates, new blocks) without expensive polling loops.
- **EIP-1559 Aware Scheduling**: Decides whether to submit, defer, or reprice transactions based on short-horizon models of base fee trends and per-block volatility.
- **Degradation / Fallback Mode**: Automatically switches to an "inclusion-first" mode during extreme gas spikes, prioritizing timely inclusion over price optimality.
- **Lock-Free Nonce Management**: Concurrent, high-throughput nonce management using atomics and sharded storage via `DashMap` to eliminate hot-path contention.
- **O(1) Rate Limiting**: Efficient, constant-time rate limiting for RPC/Exchange interactions using an atomic-backed Token Bucket algorithm.
- **Borsh Serialization**: Uses Borsh for internal message and state encoding, offering superior performance and smaller payloads compared to JSON.
- **Backpressure Support**: Built-in awareness of channel capacities and stream processing speeds to ensure stability under load.


## 🛠️ Getting Started

### Prerequisites

- [Rust](https://www.rust-lang.org/) (latest stable)
- [Cargo](https://doc.rust-lang.org/cargo/)

### Installation

```bash
git clone <repository-url>
cd gas_saver_eth
cargo build
```

### Running the Simulation

The project includes a simulation harness in `main.rs` that demonstrates how the scheduler reacts to a simulated gas spike and handles transaction repricing.

```bash
cargo run
```

## 📊 Run Tests

```bash
cargo test
```

## 🛡️ License

This project is licensed under the MIT License - see the LICENSE file for details.
