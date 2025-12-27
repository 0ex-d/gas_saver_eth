use parking_lot::RwLock;
use std::collections::VecDeque;

pub struct GasModel {
    history: RwLock<VecDeque<u64>>,
    max_history: usize,
}

impl GasModel {
    pub fn new(max_history: usize) -> Self {
        Self {
            history: RwLock::new(VecDeque::with_capacity(max_history)),
            max_history,
        }
    }

    pub fn update(&self, base_fee: u64) {
        let mut history = self.history.write();
        if history.len() >= self.max_history {
            history.pop_front();
        }
        history.push_back(base_fee);
    }

    // this is used to determine if the gas price should be increased or decreased
    pub fn get_trend(&self) -> f64 {
        let history = self.history.read();
        if history.len() < 2 {
            return 0.0;
        }

        let first = *history.front().unwrap() as f64;
        let last = *history.back().unwrap() as f64;

        (last - first) / history.len() as f64
    }

    pub fn get_volatility(&self) -> f64 {
        let history = self.history.read();
        if history.len() < 2 {
            return 0.0;
        }

        let mean = history.iter().sum::<u64>() as f64 / history.len() as f64;
        let variance = history
            .iter()
            .map(|&x| {
                let diff = x as f64 - mean;
                diff * diff
            })
            .sum::<f64>()
            / history.len() as f64;

        variance.sqrt()
    }

    // latest fee at tail of queue
    pub fn current_fee(&self) -> u64 {
        self.history.read().back().copied().unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model() {
        let model = GasModel::new(10);
        model.update(10);
        model.update(20);
        model.update(30);
        let gas_price = model.current_fee();
        let trend = model.get_trend();

        assert_eq!(gas_price, 30);
        assert_eq!(trend, 6.666666666666667);
        assert_eq!(model.get_volatility(), 0.0);
    }
}
