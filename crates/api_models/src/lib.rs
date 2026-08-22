use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub type PortfolioBalance = BTreeMap<String, Decimal>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SectorPerformanceQuery {
    pub period: String,
}

/// Stable JSON error envelope used by the canonical API.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiError {
    pub error: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarketHistory {
    pub ticker: String,
    pub currency: Option<String>,
    pub exchange_timezone: Option<String>,
    pub daily_quotes: Vec<DailyQuote>,
    pub dividends: Vec<MarketDividend>,
    pub splits: Vec<StockSplit>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DailyQuote {
    pub timestamp: DateTime<Utc>,
    pub open: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub close: Option<f64>,
    pub adjusted_close: Option<f64>,
    pub volume: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarketDividend {
    pub timestamp: DateTime<Utc>,
    pub amount: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StockSplit {
    pub timestamp: DateTime<Utc>,
    pub numerator: f64,
    pub denominator: f64,
    pub ratio: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LivePrice {
    pub ticker: String,
    pub price: f64,
    pub market_time: i64,
    pub currency: String,
    pub exchange: String,
    pub regular_session: bool,
    pub change: f64,
    pub change_percent: f64,
    pub day_volume: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OptionType {
    Call,
    Put,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OptionContract {
    pub occ_symbol: String,
    pub option_type: OptionType,
    pub strike: f64,
    pub expiration: NaiveDate,
    pub bid: f64,
    pub ask: f64,
    pub mid: f64,
    pub spread: f64,
    pub volume: f64,
    pub open_interest: Option<f64>,
    pub delta: f64,
    pub gamma: Option<f64>,
    pub vega: f64,
    pub theta: f64,
    pub rho: f64,
    pub theo: f64,
    pub implied_volatility: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OptionChain {
    pub root: String,
    pub contratos: Vec<OptionContract>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OptionSnapshot {
    pub ticker: String,
    pub timestamp_utc: DateTime<Utc>,
    pub contratos: Vec<OptionContract>,
    pub chains: Vec<OptionChain>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GammaExposureSnapshotOrigin {
    Intraday,
    EndOfDay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GammaExposureExclusionReason {
    MissingSpot,
    InvalidSpot,
    MissingGamma,
    InvalidGamma,
    MissingOpenInterest,
    InvalidOpenInterest,
    MissingMultiplier,
    InvalidMultiplier,
    InvalidStrike,
    ExpiredContract,
    MissingImpliedVolatility,
    InvalidImpliedVolatility,
    MissingForwardCarry,
    NumericOverflow,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GammaExposureStrike {
    pub strike: f64,
    pub calls_gex: f64,
    pub puts_gex: f64,
    pub net_gex: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GammaExposureExpiration {
    pub expiration: NaiveDate,
    pub calls_gex: f64,
    pub puts_gex: f64,
    pub net_gex: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GammaExposureExclusionCount {
    pub reason: GammaExposureExclusionReason,
    pub count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GammaExposureExclusionSample {
    pub occ_symbol: String,
    pub reasons: Vec<GammaExposureExclusionReason>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GammaExposureDiagnostics {
    pub total_contracts: u64,
    pub included_contracts: u64,
    pub excluded_contracts: u64,
    pub excluded_by_reason: Vec<GammaExposureExclusionCount>,
    pub exclusion_samples: Vec<GammaExposureExclusionSample>,
    pub exclusion_sample_limit: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CurrentGammaExposureResponse {
    pub ticker: String,
    pub spot: Option<f64>,
    pub currency: Option<String>,
    pub as_of: Option<DateTime<Utc>>,
    pub snapshot_origin: GammaExposureSnapshotOrigin,
    pub calls_gex: f64,
    pub puts_gex: f64,
    pub net_gex: f64,
    pub by_strike: Vec<GammaExposureStrike>,
    pub by_expiration: Vec<GammaExposureExpiration>,
    pub methodology: String,
    pub sign_convention: String,
    pub diagnostics: GammaExposureDiagnostics,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModeledGammaExposurePoint {
    pub spot: f64,
    pub call_gex: f64,
    pub put_gex: f64,
    pub net_gex: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModeledGammaExposureProfile {
    pub valuation_time: DateTime<Utc>,
    pub range_percent: f64,
    pub points: usize,
    pub methodology: String,
    pub sticky_strike_assumption: String,
    pub included_contracts: u64,
    pub excluded_contracts: u64,
    pub diagnostics: GammaExposureDiagnostics,
    pub profile: Vec<ModeledGammaExposurePoint>,
    pub zero_crossings: Vec<f64>,
    pub nearest_zero_crossing: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GammaExposureResponse {
    pub current_exposure: CurrentGammaExposureResponse,
    pub modeled_profile: DataState<ModeledGammaExposureProfile>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct GammaExposureQuery {
    pub range_percent: Option<f64>,
    pub points: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TermStructure {
    pub ticker: String,
    pub snapshot_timestamp: DateTime<Utc>,
    pub treasury_date: NaiveDate,
    pub points: Vec<TermStructurePoint>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TermStructurePoint {
    pub days: f64,
    pub variance: f64,
    pub volatility: f64,
    pub source: TermStructureSource,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TermStructureSource {
    Interpolated {
        near_expiration: NaiveDate,
        near_rate: f64,
        next_expiration: NaiveDate,
        next_rate: f64,
    },
    Expiration {
        expiration: NaiveDate,
        interest_rate: f64,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VolatilitySurface {
    pub ticker: String,
    pub snapshot_time: DateTime<Utc>,
    pub reference_price: f64,
    pub points: Vec<CanonicalVolatilitySurfacePoint>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VolatilitySkew {
    pub ticker: String,
    pub expiration: NaiveDate,
    pub points: Vec<CanonicalVolatilitySurfacePoint>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CanonicalVolatilitySurfacePoint {
    pub expiration: NaiveDate,
    pub days_to_expiration: i64,
    pub strike: f64,
    pub moneyness: f64,
    pub option_type: OptionType,
    pub implied_volatility: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExerciseStyle {
    European,
    American,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StrategyOptionContract {
    pub symbol: String,
    pub option_type: OptionType,
    pub exercise_style: ExerciseStyle,
    pub strike: f64,
    pub expiration: NaiveDate,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StrategyLeg {
    pub contract: StrategyOptionContract,
    pub quantity: i32,
    pub multiplier: u32,
    pub entry_price: f64,
    pub entry_volatility: Option<f64>,
    pub fees: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Strategy {
    pub id: Option<String>,
    pub root: String,
    pub legs: Vec<StrategyLeg>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarketState {
    pub valuation_date: NaiveDate,
    pub spot: f64,
    pub risk_free_rate: f64,
    pub dividend_yield: f64,
    pub volatility: f64,
    pub snapshot_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScenarioGrid {
    pub spots: Vec<f64>,
    pub valuation_dates: Vec<NaiveDate>,
    pub volatility_shifts: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScenarioGridRequest {
    pub spot: f64,
    pub range_fraction: f64,
    pub spot_count: usize,
    pub valuation_dates: Vec<NaiveDate>,
    pub volatility_shifts: Vec<f64>,
    pub required_spots: Vec<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PricingModel {
    BlackScholes,
    Binomial { steps: u32 },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PricingConfig {
    pub european_model: PricingModel,
    pub american_model: PricingModel,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StrategySimulationRequest {
    pub strategy: Strategy,
    pub market: MarketState,
    pub grid: ScenarioGrid,
    pub pricing: PricingConfig,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Greeks {
    pub delta: f64,
    pub gamma: f64,
    pub vega: f64,
    pub theta: f64,
    pub rho: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LegSimulationResult {
    pub symbol: String,
    pub theoretical_price: f64,
    pub position_value: f64,
    pub pnl: f64,
    pub intrinsic_value: f64,
    pub temporal_value: f64,
    pub greeks: Greeks,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SimulationWarning {
    AtOrAfterExpiration,
    VolatilityFloored,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimulationPoint {
    pub spot: f64,
    pub valuation_date: NaiveDate,
    pub volatility_shift: f64,
    pub theoretical_value: f64,
    pub pnl: f64,
    pub greeks: Greeks,
    pub legs: Vec<LegSimulationResult>,
    pub warnings: Vec<SimulationWarning>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StrategySimulationResult {
    pub strategy_id: Option<String>,
    pub model: PricingModel,
    pub points: Vec<SimulationPoint>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrackedTickerSource {
    System,
    User,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnderlyingResolutionState {
    Pending,
    Resolved,
    Rejected,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnderlyingMetadata {
    pub currency: Option<String>,
    pub exchange: Option<String>,
    pub timezone: Option<String>,
    pub instrument_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrackedTicker {
    pub ticker: String,
    pub source: TrackedTickerSource,
    pub active: bool,
    pub historical_prices: bool,
    pub option_snapshots: bool,
    pub resolution_state: UnderlyingResolutionState,
    pub validated_at: Option<DateTime<Utc>>,
    pub metadata: UnderlyingMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolveUnderlyingQuery {
    pub ticker: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnderlyingResolution {
    pub ticker: String,
    pub validated_at: DateTime<Utc>,
    pub metadata: UnderlyingMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigureTrackedTickerRequest {
    pub active: bool,
    pub historical_prices: bool,
    pub option_snapshots: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrackedTickersQuery {
    #[serde(default)]
    pub include_inactive: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SavedStrategy {
    pub id: i64,
    pub name: String,
    pub ticker: String,
    pub legs: Vec<SavedStrategyLeg>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SavedStrategyLeg {
    pub occ_symbol: String,
    pub side: SavedStrategySide,
    pub quantity: u32,
    pub entry_price: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SavedStrategySide {
    #[serde(rename = "buy", alias = "Buy")]
    Buy,
    #[serde(rename = "sell", alias = "Sell")]
    Sell,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SaveStrategy {
    pub name: String,
    pub ticker: String,
    pub legs: Vec<SavedStrategyLeg>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreatePortfolioRequest {
    pub id: String,
    pub name: String,
    pub base_currency: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Money {
    pub amount: Decimal,
    pub currency: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExchangeRate {
    pub base: String,
    pub quote: String,
    pub rate: Decimal,
    pub reference_date: NaiveDate,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Instrument {
    Equity { ticker: String },
    Option { occ_symbol: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CashMovementKind {
    Deposit,
    Withdrawal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CashMovement {
    pub id: String,
    pub occurred_at: DateTime<Utc>,
    pub kind: CashMovementKind,
    pub amount: Money,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TradeSide {
    Buy,
    Sell,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Trade {
    pub id: String,
    pub instrument: Instrument,
    pub side: TradeSide,
    pub executed_at: DateTime<Utc>,
    pub quantity: Decimal,
    pub unit_price: Money,
    pub fees: Vec<Money>,
    pub settlement_rate_to_eur: Option<ExchangeRate>,
    pub tax_rate_to_eur: Option<ExchangeRate>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CurrencyExchange {
    pub id: String,
    pub occurred_at: DateTime<Utc>,
    pub sold: Money,
    pub bought: Money,
    pub rate: ExchangeRate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortfolioDividend {
    pub id: String,
    pub instrument: Instrument,
    pub paid_at: DateTime<Utc>,
    pub gross: Money,
    pub withholding_tax: Money,
    pub tax_rate_to_eur: Option<ExchangeRate>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PortfolioEvent {
    Trade(Trade),
    CashMovement(CashMovement),
    Dividend(PortfolioDividend),
    CurrencyExchange(CurrencyExchange),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Position {
    pub instrument: Instrument,
    pub quantity: Decimal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SynchronizationReport {
    pub items_obtained: usize,
    pub items_stored: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketHistorySynchronizationRequest {
    pub since: NaiveDate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OptionChainSynchronizationRequest {
    pub market_close: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SynchronizeTrackedTickersRequest {
    pub since: NaiveDate,
    pub market_close: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SynchronizationFailure {
    pub ticker: String,
    pub operation: String,
    pub error: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrackedTickersSynchronizationReport {
    pub tickers: usize,
    pub items_obtained: usize,
    pub items_stored: u64,
    pub failures: Vec<SynchronizationFailure>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataRefreshState {
    Running,
    Completed,
    Partial,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataRefreshOrigin {
    Startup,
    Scheduled,
    Retry,
    Manual,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataRefreshFailure {
    pub ticker: String,
    pub operation: String,
    pub error: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataRefreshRun {
    pub id: String,
    pub origin: DataRefreshOrigin,
    pub state: DataRefreshState,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub target_session: NaiveDate,
    pub items_obtained: u64,
    pub items_persisted: u64,
    pub failure_count: u64,
    pub next_attempt_at: Option<DateTime<Utc>>,
    pub summary: String,
    pub failures: Vec<DataRefreshFailure>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataRefreshStatusResponse {
    pub running: bool,
    pub latest: Option<DataRefreshRun>,
    pub recent: Vec<DataRefreshRun>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataRefreshRequestState {
    Started,
    AlreadyRunning,
    NoEligibleSession,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataRefreshRequestResponse {
    pub result: DataRefreshRequestState,
    pub run: Option<DataRefreshRun>,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetKind {
    Equity,
    Etf,
    Index,
    VolatilityIndex,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssetLivePrice {
    pub ticker: String,
    pub price: f64,
    pub market_time: i64,
    pub currency: String,
    pub exchange: String,
    pub market_hours: i32,
    pub change: f64,
    pub change_percent: f64,
    pub day_volume: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ViewContext {
    Market,
    Asset,
    Options,
    Portfolio,
    Simulation,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimulationOverview {
    pub ticker: String,
    pub strategy_kind: SimulationStrategyKind,
    pub strategy_label: String,
    pub valuation_date: NaiveDate,
    pub expiration: NaiveDate,
    pub strike: f64,
    pub upper_strike: Option<f64>,
    pub spot: f64,
    pub model: String,
    pub break_even_points: Vec<f64>,
    pub curves: Vec<SimulationCurveOverview>,
    pub legs: Vec<SimulationLegOverview>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimulationScenarioRequest {
    pub ticker: String,
    pub valuation_dates: Vec<NaiveDate>,
    pub strategy_kind: SimulationStrategyKind,
    pub volatility_shifts: Vec<f64>,
    #[serde(default)]
    pub legs: Vec<SimulationLegRequest>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SimulationStrategyKind {
    Straddle,
    BullCallSpread,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SimulationTradeSide {
    Buy,
    Sell,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimulationLegRequest {
    pub occ_symbol: String,
    pub side: SimulationTradeSide,
    pub quantity: u32,
    pub entry_price: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SaveStrategyRequest {
    pub name: String,
    pub ticker: String,
    pub legs: Vec<SimulationLegRequest>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SavedStrategyOverview {
    pub id: i64,
    pub name: String,
    pub ticker: String,
    pub legs: Vec<SimulationLegRequest>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimulationLegOverview {
    pub occ_symbol: String,
    pub option_type: String,
    pub strike: f64,
    pub expiration: NaiveDate,
    pub side: SimulationTradeSide,
    pub quantity: u32,
    pub entry_price: f64,
    pub base_volatility: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimulationCatalogOverview {
    pub ticker: String,
    pub snapshot_time: DateTime<Utc>,
    pub spot: f64,
    pub expirations: Vec<NaiveDate>,
    pub contracts: Vec<SimulationContractOverview>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimulationContractOverview {
    pub occ_symbol: String,
    pub option_type: String,
    pub strike: f64,
    pub expiration: NaiveDate,
    pub bid: f64,
    pub ask: f64,
    pub mid: f64,
    pub implied_volatility: Option<f64>,
    pub delta: f64,
    pub gamma: Option<f64>,
    pub theta: f64,
    pub vega: f64,
    pub rho: f64,
    pub volume: f64,
    pub open_interest: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimulationCurveOverview {
    pub label: String,
    pub valuation_date: NaiveDate,
    pub volatility_shift: f64,
    pub volatility_limited: bool,
    pub points: Vec<SimulationPointOverview>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimulationPointOverview {
    pub spot: f64,
    pub pnl: f64,
    pub greeks: SimulationGreeksOverview,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct SimulationGreeksOverview {
    pub delta: f64,
    pub gamma: f64,
    pub theta: f64,
    pub vega: f64,
    pub rho: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Freshness {
    Current,
    Stale,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "state", content = "data", rename_all = "snake_case")]
pub enum DataState<T> {
    Available(T),
    Stale(T),
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewMetadata {
    pub session_date: NaiveDate,
    pub collected_at: Option<DateTime<Utc>>,
    pub source: String,
    pub freshness: Freshness,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetSummary {
    pub ticker: String,
    pub name: String,
    pub kind: AssetKind,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarketBenchmarkResponse {
    pub as_of: NaiveDate,
    pub benchmark: DataState<BenchmarkOverview>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarketSpxHistoryResponse {
    pub as_of: NaiveDate,
    pub spx_history: DataState<PriceHistoryOverview>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarketVolatilityResponse {
    pub as_of: NaiveDate,
    pub volatility: DataState<VolatilityOverview>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarketVixHistoryResponse {
    pub as_of: NaiveDate,
    pub vix_history: DataState<IndexHistoryOverview>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarketRatesResponse {
    pub as_of: NaiveDate,
    pub rates: DataState<RatesOverview>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SectorPerformancePeriod {
    #[serde(rename = "1w")]
    OneWeek,
    #[serde(rename = "2w")]
    TwoWeeks,
    #[serde(rename = "1m")]
    OneMonth,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarketSectorPerformanceResponse {
    pub as_of: NaiveDate,
    pub period: SectorPerformancePeriod,
    pub benchmark: DataState<SectorBenchmarkOverview>,
    pub sectors: Vec<SectorPerformanceOverview>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SectorBenchmarkOverview {
    pub metadata: ViewMetadata,
    pub ticker: String,
    pub return_percent: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SectorPerformanceOverview {
    pub name: String,
    pub etf: String,
    pub performance: DataState<SectorReturnOverview>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SectorReturnOverview {
    pub metadata: ViewMetadata,
    pub return_percent: f64,
    pub relative_strength_percentage_points: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BenchmarkOverview {
    pub metadata: ViewMetadata,
    pub ticker: String,
    pub close: f64,
    pub daily_change_pct: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VolatilityOverview {
    pub metadata: ViewMetadata,
    pub vix: IndexValue,
    pub spx_30_day: Option<CalculatedVolatility>,
    pub vvix: Option<IndexValue>,
    pub term_structure: Vec<IndexValue>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CalculatedVolatility {
    pub ticker: String,
    pub snapshot_timestamp: DateTime<Utc>,
    pub volatility_percent: f64,
    pub difference_from_vix: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndexValue {
    pub ticker: String,
    pub date: NaiveDate,
    pub close: f64,
    pub daily_change_pct: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndexHistoryOverview {
    pub metadata: ViewMetadata,
    pub ticker: String,
    pub points: Vec<IndexHistoryPoint>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndexHistoryPoint {
    pub date: NaiveDate,
    pub close: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RatesOverview {
    pub metadata: ViewMetadata,
    pub points: Vec<RatePoint>,
    pub interpolated_points: Vec<InterpolatedRatePoint>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RatePoint {
    pub tenor: String,
    pub days: f64,
    pub rate_percent: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InterpolatedRatePoint {
    pub days: f64,
    pub rate_percent: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssetPriceResponse {
    pub ticker: String,
    pub as_of: Option<NaiveDate>,
    pub price: DataState<AssetPriceOverview>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssetPriceHistoryResponse {
    pub ticker: String,
    pub as_of: Option<NaiveDate>,
    pub price_history: DataState<PriceHistoryOverview>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssetHistoricalVolatilityResponse {
    pub ticker: String,
    pub as_of: Option<NaiveDate>,
    pub historical_volatility: DataState<HistoricalVolatilityOverview>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssetImpliedVolatilityResponse {
    pub ticker: String,
    pub implied_volatility: DataState<ImpliedVolatilityOverview>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PriceHistoryOverview {
    pub metadata: ViewMetadata,
    pub points: Vec<PriceHistoryPoint>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PriceHistoryPoint {
    pub date: NaiveDate,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistoricalVolatilityOverview {
    pub metadata: ViewMetadata,
    pub points: Vec<HistoricalVolatilityPoint>,
    pub series: Vec<HistoricalVolatilitySeriesPoint>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistoricalVolatilityPoint {
    pub window_sessions: usize,
    pub observations: usize,
    pub annualized_volatility_percent: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistoricalVolatilitySeriesPoint {
    pub date: NaiveDate,
    pub window_sessions: usize,
    pub annualized_volatility_percent: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImpliedVolatilityOverview {
    pub metadata: ViewMetadata,
    pub reference_ticker: Option<String>,
    pub points: Vec<ImpliedVolatilityPoint>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImpliedVolatilityPoint {
    pub date: NaiveDate,
    pub volatility_percent: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssetPriceOverview {
    pub metadata: ViewMetadata,
    pub open: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub close: f64,
    pub adjusted_close: Option<f64>,
    pub volume: Option<u64>,
    pub daily_change_pct: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PortfolioOverview {
    pub id: String,
    pub name: String,
    pub base_currency: String,
    pub event_count: usize,
    pub realized_gain_eur: String,
    pub net_value_eur: Option<String>,
    pub valuation_note: Option<String>,
    pub realized_gains: Vec<PortfolioCashOverview>,
    pub positions: Vec<PortfolioPositionOverview>,
    pub cash_balances: Vec<PortfolioCashOverview>,
    pub movements: Vec<PortfolioMovementOverview>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PortfolioSummaryResponse {
    pub id: String,
    pub name: String,
    pub base_currency: String,
    pub event_count: usize,
    pub realized_gain_eur: String,
    pub net_value_eur: Option<String>,
    pub valuation_note: Option<String>,
    pub realized_gains: Vec<PortfolioCashOverview>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PortfolioCashResponse {
    pub cash_balances: Vec<PortfolioCashOverview>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PortfolioPositionsResponse {
    pub positions: Vec<PortfolioPositionOverview>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PortfolioMovementsResponse {
    pub movements: Vec<PortfolioMovementOverview>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PortfolioPositionOverview {
    pub instrument: String,
    pub quantity: String,
    pub market_price: Option<f64>,
    pub market_value: Option<f64>,
    pub market_currency: Option<String>,
    pub market_source: Option<String>,
    pub market_time: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PortfolioCashOverview {
    pub currency: String,
    pub amount: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PortfolioMovementOverview {
    pub id: String,
    pub occurred_at: DateTime<Utc>,
    pub kind: String,
    pub description: String,
    pub amount: String,
    pub currency: String,
    pub counter_amount: Option<String>,
    pub counter_currency: Option<String>,
    pub tax_amount_eur: Option<String>,
    pub tax_rate: Option<String>,
    pub tax_rate_date: Option<NaiveDate>,
    pub tax_rate_source: Option<String>,
    pub exchange_rate_label: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PortfolioCashMovementKind {
    Deposit,
    Withdrawal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreatePortfolioCashMovement {
    pub id: String,
    pub occurred_at: DateTime<Utc>,
    pub kind: PortfolioCashMovementKind,
    pub amount: String,
    pub currency: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreatePortfolioOptionTrade {
    pub id: String,
    pub occurred_at: DateTime<Utc>,
    pub occ_symbol: String,
    pub side: PortfolioTradeSide,
    pub quantity: String,
    pub premium: String,
    pub currency: String,
    pub tax_rate_to_eur: String,
    pub tax_rate_date: NaiveDate,
    pub tax_rate_source: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PortfolioTradeSide {
    Buy,
    Sell,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreatePortfolioCurrencyExchange {
    pub id: String,
    pub occurred_at: DateTime<Utc>,
    pub sold_amount: String,
    pub sold_currency: String,
    pub bought_amount: String,
    pub bought_currency: String,
    pub rate: String,
    pub rate_date: NaiveDate,
    pub rate_source: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OptionsSnapshotResponse {
    pub ticker: String,
    pub snapshot_time: Option<DateTime<Utc>>,
    pub snapshot: DataState<OptionSnapshotOverview>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OptionsTermStructureResponse {
    pub ticker: String,
    pub snapshot_time: Option<DateTime<Utc>>,
    pub term_structure: DataState<OptionTermStructureOverview>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OptionsVolatilitySurfaceResponse {
    pub ticker: String,
    pub snapshot_time: Option<DateTime<Utc>>,
    pub volatility_surface: DataState<VolatilitySurfaceOverview>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OptionsIntradayResponse {
    pub ticker: String,
    pub snapshot_time: DateTime<Utc>,
    pub catalog: SimulationCatalogOverview,
    pub volatility_surface: DataState<VolatilitySurfaceOverview>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OptionSnapshotOverview {
    pub metadata: ViewMetadata,
    pub expirations: usize,
    pub contracts: usize,
    pub calls: usize,
    pub puts: usize,
    pub minimum_strike: Option<f64>,
    pub maximum_strike: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OptionTermStructureOverview {
    pub metadata: ViewMetadata,
    pub treasury_date: NaiveDate,
    pub points: Vec<OptionTermStructurePoint>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OptionTermStructurePoint {
    pub days: f64,
    pub volatility_percent: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VolatilitySurfaceOverview {
    pub metadata: ViewMetadata,
    pub reference_price: f64,
    pub expirations: Vec<VolatilitySurfaceExpiration>,
    pub moneyness_levels: Vec<f64>,
    pub points: Vec<VolatilitySurfacePoint>,
    pub observations: Vec<VolatilitySurfaceObservation>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VolatilitySurfaceObservation {
    pub expiration: NaiveDate,
    pub days: i64,
    pub strike: f64,
    pub moneyness_percent: f64,
    pub volatility_percent: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VolatilitySurfaceExpiration {
    pub date: NaiveDate,
    pub days: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VolatilitySurfacePoint {
    pub expiration: NaiveDate,
    pub days: i64,
    pub strike: f64,
    pub moneyness_percent: f64,
    pub volatility_percent: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DataRefreshReport {
    pub completed_at: DateTime<Utc>,
    pub tickers: usize,
    pub price_rows: u64,
    pub snapshots: usize,
    pub term_structure_points: u64,
    pub index_rows: u64,
    pub treasury_rows: u64,
    pub failures: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_state_has_an_explicit_wire_representation() {
        let state = DataState::Stale(17.25_f64);

        let json = serde_json::to_string(&state).expect("o estado deve ser serializável");
        let decoded: DataState<f64> =
            serde_json::from_str(&json).expect("o estado deve ser desserializável");

        assert_eq!(json, r#"{"state":"stale","data":17.25}"#);
        assert_eq!(decoded, state);
    }

    #[test]
    fn sector_periods_have_stable_wire_values() {
        assert_eq!(
            serde_json::to_string(&SectorPerformancePeriod::OneWeek).unwrap(),
            "\"1w\""
        );
        assert_eq!(
            serde_json::to_string(&SectorPerformancePeriod::TwoWeeks).unwrap(),
            "\"2w\""
        );
        assert_eq!(
            serde_json::to_string(&SectorPerformancePeriod::OneMonth).unwrap(),
            "\"1m\""
        );
    }
}
