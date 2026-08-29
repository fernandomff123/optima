use crate::domain::asset::{AssetCapability, AssetSymbol};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OptionsScenario {
    Normal,
    Loading,
    Unavailable,
    RecoverableError,
}

impl OptionsScenario {
    pub fn from_query(value: Option<&str>) -> Self {
        match value {
            Some("loading") => Self::Loading,
            Some("unavailable") => Self::Unavailable,
            Some("recoverable-error") => Self::RecoverableError,
            _ => Self::Normal,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct OptionSideSnapshot {
    pub last: &'static str,
    pub bid: &'static str,
    pub ask: &'static str,
    pub iv: &'static str,
    pub delta: &'static str,
    pub open_interest: &'static str,
    pub volume: &'static str,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OptionChainRowSnapshot {
    pub strike: &'static str,
    pub is_atm: bool,
    pub is_selected: bool,
    pub call: OptionSideSnapshot,
    pub put: OptionSideSnapshot,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SmilePointSnapshot {
    pub strike: f64,
    pub call_iv: f64,
    pub put_iv: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ContractDetailSnapshot {
    pub title: &'static str,
    pub price: &'static str,
    pub change: &'static str,
    pub bid: &'static str,
    pub ask: &'static str,
    pub bid_size: &'static str,
    pub ask_size: &'static str,
    pub metrics: Vec<(&'static str, &'static str)>,
    pub facts: Vec<(&'static str, &'static str)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AssetOptionsSnapshot {
    pub symbol: AssetSymbol,
    pub name: &'static str,
    pub venue: &'static str,
    pub price: &'static str,
    pub spot: f64,
    pub absolute_change: &'static str,
    pub percentage_change: &'static str,
    pub change_positive: bool,
    pub capabilities: Vec<AssetCapability>,
    pub expiration: &'static str,
    pub dte: &'static str,
    pub strike_range: &'static str,
    pub iv_rank: &'static str,
    pub put_call_oi: &'static str,
    pub earnings: &'static str,
    pub chain: Vec<OptionChainRowSnapshot>,
    pub smile: Vec<SmilePointSnapshot>,
    pub contract: ContractDetailSnapshot,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AssetOptionsFailure {
    Recoverable,
}

pub trait AssetOptionsPort {
    fn load(
        &self,
        symbol: &AssetSymbol,
        scenario: OptionsScenario,
    ) -> Result<Option<AssetOptionsSnapshot>, AssetOptionsFailure>;
}
