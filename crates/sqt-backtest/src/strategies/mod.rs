//! Built-in strategy implementations.

pub mod bollinger_reversion;
pub mod macd_crossover;
pub mod rsi_mean_reversion;
pub mod sma_crossover;

pub use bollinger_reversion::BollingerReversion;
pub use macd_crossover::MacdCrossover;
pub use rsi_mean_reversion::RsiMeanReversion;
pub use sma_crossover::SmaCrossover;
