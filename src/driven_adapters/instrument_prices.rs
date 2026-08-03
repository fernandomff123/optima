//! Composite driven adapter selecting current or stored instrument prices.

use std::collections::BTreeMap;

use sqlx::SqlitePool;

use crate::{
    driven_adapters::{cboe, exchange_calendar, sqlite, yahoo},
    hexagon::{
        PortError, PortResult,
        domain::{options::Snapshot, portfolio::Instrument, portfolio_valuation::InstrumentPrice},
        driven_ports::for_obtaining_instrument_prices::ForObtainingInstrumentPrices,
    },
};

#[derive(Clone)]
pub struct MarketInstrumentPricesAdapter {
    pool: SqlitePool,
}

impl MarketInstrumentPricesAdapter {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl ForObtainingInstrumentPrices for MarketInstrumentPricesAdapter {
    async fn obtain_instrument_prices(
        &self,
        instruments: &[Instrument],
    ) -> PortResult<Vec<Option<InstrumentPrice>>> {
        let regular_session =
            exchange_calendar::is_regular_session(chrono::Utc::now()).map_err(unavailable)?;
        let mut option_snapshots = BTreeMap::new();
        let mut prices = Vec::with_capacity(instruments.len());
        for instrument in instruments {
            prices.push(
                self.price(instrument, regular_session, &mut option_snapshots)
                    .await
                    .unwrap_or(None),
            );
        }
        Ok(prices)
    }
}

impl MarketInstrumentPricesAdapter {
    async fn price(
        &self,
        instrument: &Instrument,
        regular_session: bool,
        option_snapshots: &mut BTreeMap<String, Snapshot>,
    ) -> PortResult<Option<InstrumentPrice>> {
        match instrument {
            Instrument::Equity { ticker } if regular_session => {
                let price = yahoo::live::fetch_price(ticker)
                    .await
                    .map_err(unavailable)?;
                Ok(Some(InstrumentPrice {
                    price: price.price,
                    currency: price.currency,
                    source: "Yahoo Finance intradiário".to_string(),
                    observed_at: chrono::DateTime::from_timestamp(price.time, 0)
                        .unwrap_or_else(chrono::Utc::now),
                }))
            }
            Instrument::Equity { ticker } => {
                let history = sqlite::market_history::load_history(&self.pool, ticker)
                    .await
                    .map_err(unavailable)?;
                Ok(history.daily_quotes.last().and_then(|quote| {
                    Some(InstrumentPrice {
                        price: quote.close?,
                        currency: history
                            .currency
                            .clone()
                            .unwrap_or_else(|| "USD".to_string()),
                        source: "Yahoo Finance EOD".to_string(),
                        observed_at: quote.timestamp,
                    })
                }))
            }
            Instrument::Option { occ_symbol } => {
                let occ = cboe::parse_occ_symbol(occ_symbol).map_err(unavailable)?;
                let ticker = if occ.root.eq_ignore_ascii_case("SPXW") {
                    "SPX".to_string()
                } else {
                    occ.root
                };
                if !option_snapshots.contains_key(&ticker) {
                    let snapshot = if regular_session {
                        let response = cboe::download_snapshot(&ticker)
                            .await
                            .map_err(unavailable)?;
                        cboe::response_to_snapshot(&ticker, response).map_err(unavailable)?
                    } else {
                        let Some(snapshot) =
                            sqlite::option_snapshots::load_latest(&self.pool, &ticker)
                                .await
                                .map_err(unavailable)?
                        else {
                            return Ok(None);
                        };
                        snapshot
                    };
                    option_snapshots.insert(ticker.clone(), snapshot);
                }
                let snapshot = option_snapshots.get(&ticker).ok_or_else(|| {
                    PortError::Unavailable("option snapshot cache is inconsistent".to_string())
                })?;
                Ok(snapshot
                    .contratos
                    .iter()
                    .find(|contract| contract.occ_symbol == *occ_symbol)
                    .map(|contract| InstrumentPrice {
                        price: contract.mid,
                        currency: "USD".to_string(),
                        source: if regular_session {
                            "CBOE intradiário".to_string()
                        } else {
                            "CBOE EOD".to_string()
                        },
                        observed_at: snapshot.timestamp_utc,
                    }))
            }
        }
    }
}

fn unavailable(error: impl std::fmt::Display) -> PortError {
    PortError::Unavailable(error.to_string())
}
