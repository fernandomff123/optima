use crate::{
    domain::asset::{AssetCapability, AssetSymbol},
    ports::asset_options::{
        AssetOptionsFailure, AssetOptionsPort, AssetOptionsSnapshot, ContractDetailSnapshot,
        OptionChainRowSnapshot, OptionSideSnapshot, OptionsScenario, SmilePointSnapshot,
    },
};
use std::rc::Rc;

mod selection;
pub use selection::{OptionKind, OptionQuote, OptionSelection};

#[derive(Clone, Debug, PartialEq)]
pub struct OptionSide {
    pub last: String,
    pub change: String,
    pub bid: String,
    pub ask: String,
    pub mid: String,
    pub bid_size: String,
    pub ask_size: String,
    pub last_size: String,
    pub iv: String,
    pub delta: String,
    pub gamma: String,
    pub vega: String,
    pub theta: String,
    pub rho: String,
    pub open_interest: String,
    pub volume: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OptionChainRow {
    pub strike: String,
    pub is_atm: bool,
    pub is_selected: bool,
    pub call: OptionSide,
    pub put: OptionSide,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VolatilitySmile {
    pub strikes: Vec<f64>,
    pub call_iv: Vec<f64>,
    pub put_iv: Vec<f64>,
    pub spot: f64,
    pub spot_label: String,
    pub description: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ContractDetail {
    pub title: String,
    pub price: String,
    pub change: String,
    pub bid: String,
    pub ask: String,
    pub bid_size: String,
    pub ask_size: String,
    pub action: String,
    pub position: String,
    pub selected_quote: String,
    pub quantity: i32,
    pub metrics: Vec<(String, String)>,
    pub facts: Vec<(String, String)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AssetOptionsReadModel {
    pub symbol: String,
    pub name: String,
    pub venue: String,
    pub price: String,
    pub absolute_change: String,
    pub percentage_change: String,
    pub change_positive: bool,
    pub capabilities: Vec<AssetCapability>,
    pub expiration: String,
    pub dte: String,
    pub strike_range: String,
    pub iv_rank: String,
    pub put_call_oi: String,
    pub earnings: String,
    pub chain: Vec<OptionChainRow>,
    pub smile: VolatilitySmile,
    pub contract: ContractDetail,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AssetOptionsState {
    Loading,
    Ready(AssetOptionsReadModel),
    Unavailable { symbol: String },
    RecoverableError { symbol: String },
}

#[derive(Clone)]
pub struct AssetOptionsUseCase {
    port: Rc<dyn AssetOptionsPort>,
}

impl AssetOptionsUseCase {
    pub fn new(port: Rc<dyn AssetOptionsPort>) -> Self {
        Self { port }
    }

    pub fn execute(&self, ticker: &str, scenario: OptionsScenario) -> AssetOptionsState {
        let symbol = AssetSymbol::new(ticker);
        if scenario == OptionsScenario::Loading {
            return AssetOptionsState::Loading;
        }
        match self.port.load(&symbol, scenario) {
            Ok(Some(snapshot)) => AssetOptionsState::Ready(to_read_model(snapshot)),
            Ok(None) => AssetOptionsState::Unavailable {
                symbol: symbol.as_str().to_owned(),
            },
            Err(AssetOptionsFailure::Recoverable) => AssetOptionsState::RecoverableError {
                symbol: symbol.as_str().to_owned(),
            },
        }
    }
}

fn to_read_model(snapshot: AssetOptionsSnapshot) -> AssetOptionsReadModel {
    let smile = smile(snapshot.smile, snapshot.spot, snapshot.price);
    AssetOptionsReadModel {
        symbol: snapshot.symbol.as_str().to_owned(),
        name: snapshot.name.into(),
        venue: snapshot.venue.into(),
        price: snapshot.price.into(),
        absolute_change: snapshot.absolute_change.into(),
        percentage_change: snapshot.percentage_change.into(),
        change_positive: snapshot.change_positive,
        capabilities: snapshot.capabilities,
        expiration: snapshot.expiration.into(),
        dte: snapshot.dte.into(),
        strike_range: snapshot.strike_range.into(),
        iv_rank: snapshot.iv_rank.into(),
        put_call_oi: snapshot.put_call_oi.into(),
        earnings: snapshot.earnings.into(),
        chain: snapshot.chain.into_iter().map(chain_row).collect(),
        smile,
        contract: contract(snapshot.contract),
    }
}

fn side(value: OptionSideSnapshot) -> OptionSide {
    OptionSide {
        last: value.last.into(),
        change: value.change.into(),
        bid: value.bid.into(),
        ask: value.ask.into(),
        mid: value.mid.into(),
        bid_size: value.bid_size.into(),
        ask_size: value.ask_size.into(),
        last_size: value.last_size.into(),
        iv: value.iv.into(),
        delta: value.delta.into(),
        gamma: value.gamma.into(),
        vega: value.vega.into(),
        theta: value.theta.into(),
        rho: value.rho.into(),
        open_interest: value.open_interest.into(),
        volume: value.volume.into(),
    }
}

fn chain_row(value: OptionChainRowSnapshot) -> OptionChainRow {
    OptionChainRow {
        strike: value.strike.into(),
        is_atm: value.is_atm,
        is_selected: value.is_selected,
        call: side(value.call),
        put: side(value.put),
    }
}

fn smile(values: Vec<SmilePointSnapshot>, spot: f64, spot_text: &str) -> VolatilitySmile {
    let mut strikes = Vec::with_capacity(values.len());
    let mut call_iv = Vec::with_capacity(values.len());
    let mut put_iv = Vec::with_capacity(values.len());
    for value in values {
        strikes.push(value.strike);
        call_iv.push(value.call_iv);
        put_iv.push(value.put_iv);
    }
    VolatilitySmile {
        strikes,
        call_iv,
        put_iv,
        spot,
        spot_label: spot_text.into(),
        description:
            "Illustrative call and put implied-volatility smile for the selected mock expiration."
                .into(),
    }
}

fn contract(value: ContractDetailSnapshot) -> ContractDetail {
    ContractDetail {
        title: value.title.into(),
        price: value.price.into(),
        change: value.change.into(),
        bid: value.bid.into(),
        ask: value.ask.into(),
        bid_size: value.bid_size.into(),
        ask_size: value.ask_size.into(),
        action: "BUY".into(),
        position: "LONG".into(),
        selected_quote: "Ask".into(),
        quantity: 1,
        metrics: value
            .metrics
            .into_iter()
            .map(|(label, value)| (label.into(), value.into()))
            .collect(),
        facts: value
            .facts
            .into_iter()
            .map(|(label, value)| (label.into(), value.into()))
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    struct RecordingPort(Cell<usize>);
    impl AssetOptionsPort for RecordingPort {
        fn load(
            &self,
            symbol: &AssetSymbol,
            _: OptionsScenario,
        ) -> Result<Option<AssetOptionsSnapshot>, AssetOptionsFailure> {
            self.0.set(self.0.get() + 1);
            Ok(Some(AssetOptionsSnapshot {
                symbol: symbol.clone(),
                name: "Apple Inc.",
                venue: "NASDAQ",
                price: "191.13",
                spot: 191.13,
                absolute_change: "+2.34",
                percentage_change: "+1.24%",
                change_positive: true,
                capabilities: vec![AssetCapability::Options],
                expiration: "17 May 2025",
                dte: "36",
                strike_range: "±10 ATM",
                iv_rank: "42.3",
                put_call_oi: "0.79",
                earnings: "May 1, 2025",
                chain: vec![],
                smile: vec![SmilePointSnapshot {
                    strike: 190.0,
                    call_iv: 20.0,
                    put_iv: 19.0,
                }],
                contract: ContractDetailSnapshot {
                    title: "AAPL CALL",
                    price: "1.15",
                    change: "+0.14",
                    bid: "1.08",
                    ask: "1.22",
                    bid_size: "152",
                    ask_size: "163",
                    metrics: vec![],
                    facts: vec![],
                },
            }))
        }
    }

    #[test]
    fn use_case_reads_mock_through_port_and_builds_provider_neutral_smile() {
        let port = Rc::new(RecordingPort(Cell::new(0)));
        let AssetOptionsState::Ready(model) =
            AssetOptionsUseCase::new(port.clone()).execute("aapl", OptionsScenario::Normal)
        else {
            panic!("ready expected")
        };
        assert_eq!(port.0.get(), 1);
        assert_eq!(model.symbol, "AAPL");
        assert_eq!(model.smile.strikes, vec![190.0]);
        assert_eq!(model.smile.call_iv, vec![20.0]);
    }

    #[test]
    fn loading_does_not_call_port() {
        let port = Rc::new(RecordingPort(Cell::new(0)));
        assert_eq!(
            AssetOptionsUseCase::new(port.clone()).execute("AAPL", OptionsScenario::Loading),
            AssetOptionsState::Loading
        );
        assert_eq!(port.0.get(), 0);
    }

    #[test]
    fn bid_and_ask_encode_short_and_long_simulation_quantities() {
        assert_eq!(OptionQuote::Bid.action(), "SELL");
        assert_eq!(OptionQuote::Bid.position(), "SHORT");
        assert_eq!(OptionQuote::Bid.quantity(), -1);
        assert_eq!(OptionQuote::Ask.action(), "BUY");
        assert_eq!(OptionQuote::Ask.position(), "LONG");
        assert_eq!(OptionQuote::Ask.quantity(), 1);
    }
}
