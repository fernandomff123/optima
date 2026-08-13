use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

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
    pub gamma: f64,
    pub theta: f64,
    pub vega: f64,
    pub rho: f64,
    pub volume: f64,
    pub open_interest: f64,
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
