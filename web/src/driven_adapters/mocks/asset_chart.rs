use crate::{
    domain::asset::{AssetCapability, AssetSymbol},
    ports::asset_chart::{
        AssetChartFailure, AssetChartPort, AssetChartSnapshot, ChartCandleSnapshot, ChartScenario,
    },
};

#[derive(Clone, Copy, Debug, Default)]
pub struct MockAssetChartAdapter;

impl AssetChartPort for MockAssetChartAdapter {
    fn load(
        &self,
        symbol: &AssetSymbol,
        scenario: ChartScenario,
    ) -> Result<Option<AssetChartSnapshot>, AssetChartFailure> {
        if scenario == ChartScenario::RecoverableError {
            return Err(AssetChartFailure::Recoverable);
        }
        if scenario == ChartScenario::Unavailable || symbol.as_str() != "AAPL" {
            return Ok(None);
        }
        Ok(Some(aapl_snapshot()))
    }
}

fn aapl_snapshot() -> AssetChartSnapshot {
    AssetChartSnapshot {
        symbol: AssetSymbol::new("AAPL"),
        name: "Apple Inc.",
        venue: "NASDAQ",
        price: "191.13",
        absolute_change: "+2.35",
        percentage_change: "+1.24%",
        change_positive: true,
        market_status: "MARKET OPEN",
        capabilities: vec![
            AssetCapability::Overview,
            AssetCapability::Chart,
            AssetCapability::Options,
            AssetCapability::Volatility,
            AssetCapability::Gex,
            AssetCapability::Simulation,
        ],
        candles: aapl_candles(),
        average_volume: 48_670_000.0,
    }
}

fn aapl_candles() -> Vec<ChartCandleSnapshot> {
    let anchors = [
        186.0, 178.2, 181.5, 180.3, 184.0, 189.0, 195.4, 187.7, 181.2, 184.4, 177.4, 173.8, 182.4,
        186.1, 181.4, 179.0, 188.0, 191.2,
    ];
    let mut candles = (0..120)
        .map(|index| {
            let progress = index as f64 * (anchors.len() - 1) as f64 / 119.0;
            let left = progress.floor() as usize;
            let right = (left + 1).min(anchors.len() - 1);
            let fraction = progress - left as f64;
            let trend = anchors[left] + (anchors[right] - anchors[left]) * fraction;
            let close = trend + (index as f64 * 0.73).sin() * 0.72;
            let open = close + (index as f64 * 1.19).sin() * 0.9;
            let wick = 0.55 + (index as f64 * 0.41).cos().abs() * 0.65;
            let volume = 33_000_000.0
                + (index as f64 * 0.31).sin().abs() * 22_000_000.0
                + if matches!(index, 30 | 63 | 78 | 101) {
                    24_000_000.0
                } else {
                    0.0
                };
            ChartCandleSnapshot {
                timestamp: timestamp(index),
                open,
                high: open.max(close) + wick,
                low: open.min(close) - wick,
                close,
                volume,
            }
        })
        .collect::<Vec<_>>();
    candles[119] = ChartCandleSnapshot {
        timestamp: "May 21".into(),
        open: 189.44,
        high: 191.88,
        low: 188.92,
        close: 191.13,
        volume: 55_210_000.0,
    };
    candles
}

fn timestamp(index: usize) -> String {
    const MONTHS: [&str; 6] = ["Dec", "Jan", "Feb", "Mar", "Apr", "May"];
    format!("{} {:02}", MONTHS[index / 20], 2 + index % 20)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aapl_fixture_is_dense_coherent_and_keeps_visible_price_movement() {
        let candles = aapl_candles();
        assert_eq!(candles.len(), 120);
        assert!(candles.iter().all(|candle| {
            candle.high >= candle.open.max(candle.close)
                && candle.low <= candle.open.min(candle.close)
                && candle.volume > 0.0
        }));
        let low = candles
            .iter()
            .map(|candle| candle.low)
            .fold(f64::MAX, f64::min);
        let high = candles
            .iter()
            .map(|candle| candle.high)
            .fold(f64::MIN, f64::max);
        assert!(high - low > 20.0);
        assert_eq!(candles.last().unwrap().close, 191.13);
        assert_eq!(candles.last().unwrap().volume, 55_210_000.0);
    }

    #[test]
    fn unsupported_assets_and_explicit_scenarios_do_not_invent_market_data() {
        let adapter = MockAssetChartAdapter;
        assert!(
            adapter
                .load(&AssetSymbol::new("NVDA"), ChartScenario::Normal)
                .unwrap()
                .is_none()
        );
        assert!(
            adapter
                .load(&AssetSymbol::new("AAPL"), ChartScenario::Unavailable)
                .unwrap()
                .is_none()
        );
        assert_eq!(
            adapter.load(&AssetSymbol::new("AAPL"), ChartScenario::RecoverableError),
            Err(AssetChartFailure::Recoverable)
        );
    }
}
