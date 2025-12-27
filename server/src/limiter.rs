use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

pub struct RateLimiter {
    tokens: AtomicU64,
    max_tokens: u64,
    refill_rate: u64,       // tokens per second
    last_refill: AtomicU64, // Instant as nanos
}

impl RateLimiter {
    pub fn new(rate: u64, max: u64) -> Self {
        Self {
            tokens: AtomicU64::new(max),
            max_tokens: max,
            refill_rate: rate,
            last_refill: AtomicU64::new(Instant::now().elapsed().as_nanos() as u64),
        }
    }

    pub fn check_and_consume(&self) -> bool {
        self.refill();

        loop {
            let current = self.tokens.load(Ordering::SeqCst);
            if current == 0 {
                return false;
            }
            if self
                .tokens
                .compare_exchange(current, current - 1, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                return true;
            }
        }
    }

    fn refill(&self) {
        let now = Instant::now().elapsed().as_nanos() as u64;
        let last = self.last_refill.load(Ordering::SeqCst);
        let elapsed_ns = now.saturating_sub(last);

        if elapsed_ns > 1_000_000_000 {
            // at least 1 second
            let tokens_to_add = (elapsed_ns / 1_000_000_000) * self.refill_rate;
            if tokens_to_add > 0 {
                if self
                    .last_refill
                    .compare_exchange(last, now, Ordering::SeqCst, Ordering::SeqCst)
                    .is_ok()
                {
                    let mut current = self.tokens.load(Ordering::SeqCst);
                    loop {
                        let next = (current + tokens_to_add).min(self.max_tokens);
                        match self.tokens.compare_exchange(
                            current,
                            next,
                            Ordering::SeqCst,
                            Ordering::SeqCst,
                        ) {
                            Ok(_) => break,
                            Err(actual) => current = actual,
                        }
                    }
                }
            }
        }
    }
}
