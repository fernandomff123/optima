use crate::{
    domain::asset::{AssetCapability, AssetSymbol},
    ports::asset_chart::{
        AssetChartFailure, AssetChartPort, AssetChartSnapshot, CandleSnapshot, ChartScenario,
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
        match scenario {
            ChartScenario::Unavailable => return Ok(None),
            ChartScenario::RecoverableError => return Err(AssetChartFailure::Recoverable),
            _ => {}
        }
        if symbol.as_str() != "AAPL" {
            return Ok(None);
        }
        Ok(Some(AssetChartSnapshot {
            symbol: symbol.clone(),
            name: "Apple Inc.",
            venue: "NASDAQ",
            price: "191.13",
            absolute_change: "+2.35",
            percentage_change: "+1.24%",
            market_status: "MARKET OPEN",
            capabilities: vec![
                AssetCapability::Overview,
                AssetCapability::Chart,
                AssetCapability::Options,
                AssetCapability::Volatility,
                AssetCapability::Gex,
                AssetCapability::Simulation,
            ],
            candles: candles(),
        }))
    }
}

fn candles() -> Vec<CandleSnapshot> {
    // Fixed visual fixtures. The deliberately broad bodies and wicks remain legible at desktop density.
    let rows = [
        ("Dec 02", [184.2, 186.8, 182.9, 188.0], 38),
        ("Dec 09", [186.7, 183.5, 181.8, 187.4], 44),
        ("Dec 16", [183.4, 179.6, 177.8, 184.6], 57),
        ("Dec 23", [179.5, 182.1, 178.4, 183.8], 35),
        ("Dec 30", [182.0, 180.1, 178.9, 183.2], 31),
        ("Jan 06", [180.2, 184.0, 179.5, 185.1], 46),
        ("Jan 13", [184.1, 181.7, 180.5, 185.0], 41),
        ("Jan 20", [181.8, 185.3, 180.9, 186.1], 36),
        ("Jan 27", [185.2, 188.6, 184.4, 189.4], 49),
        ("Feb 03", [188.5, 191.9, 187.8, 192.8], 52),
        ("Feb 10", [192.0, 195.1, 191.2, 196.0], 61),
        ("Feb 17", [195.0, 191.6, 190.8, 196.1], 58),
        ("Feb 24", [191.5, 187.4, 186.1, 192.4], 55),
        ("Mar 03", [187.5, 183.2, 181.9, 188.3], 68),
        ("Mar 10", [183.1, 179.8, 178.2, 184.0], 64),
        ("Mar 17", [179.7, 182.6, 178.8, 183.9], 47),
        ("Mar 24", [182.7, 178.1, 176.5, 183.4], 72),
        ("Mar 31", [178.0, 173.8, 171.4, 179.1], 96),
        ("Apr 07", [173.7, 178.9, 172.6, 180.2], 84),
        ("Apr 14", [179.0, 176.4, 175.0, 180.1], 62),
        ("Apr 21", [176.5, 181.2, 175.7, 182.5], 71),
        ("Apr 28", [181.1, 185.0, 180.3, 186.2], 76),
        ("May 05", [185.1, 183.4, 182.0, 186.0], 54),
        ("May 12", [183.5, 187.7, 182.8, 188.9], 66),
        ("May 19", [187.6, 190.2, 186.8, 191.5], 59),
        ("May 26", [190.1, 188.0, 187.2, 191.0], 52),
        ("Jun 02", [188.1, 192.0, 187.5, 193.0], 63),
        ("Jun 09", [192.1, 189.2, 188.2, 193.1], 58),
        ("Jun 16", [189.1, 193.3, 188.4, 194.2], 69),
        ("Jun 23", [193.2, 190.0, 189.0, 194.0], 64),
        ("Jun 30", [190.1, 194.0, 189.4, 195.1], 73),
        ("Jul 07", [194.1, 191.13, 190.1, 195.0], 55),
    ];
    rows.into_iter()
        .map(|(date, ohlc, volume)| CandleSnapshot {
            date,
            ohlc,
            volume: volume * 1_000_000,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fixture_is_repeatable_legible_and_aapl_only() {
        let adapter = MockAssetChartAdapter;
        let symbol = AssetSymbol::new("AAPL");
        let one = adapter
            .load(&symbol, ChartScenario::Normal)
            .unwrap()
            .unwrap();
        let two = adapter
            .load(&symbol, ChartScenario::Normal)
            .unwrap()
            .unwrap();
        assert_eq!(one, two);
        assert_eq!(one.candles.len(), 32);
        assert!(one.candles.iter().all(|p| p.ohlc[3] - p.ohlc[2] >= 1.8));
        assert!(
            adapter
                .load(&AssetSymbol::new("MSFT"), ChartScenario::Normal)
                .unwrap()
                .is_none()
        );
    }
}
