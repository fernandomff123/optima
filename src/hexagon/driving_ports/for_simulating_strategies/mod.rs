//! Conversation offered to actors that analyze strategy scenarios.

use async_trait::async_trait;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::hexagon::{
    PortResult,
    domain::{
        options::Snapshot,
        simulation::{
            ScenarioGrid, SimulationCatalog, SimulationLegSelection, SimulationRequest,
            SimulationResult, SimulationScenario, SimulationStrategyKind,
        },
    },
};

/// Complete, infrastructure-free command for preparing a strategy scenario.
#[derive(Debug, Clone, PartialEq)]
pub struct SimulateScenario {
    pub ticker: String,
    pub snapshot: Snapshot,
    pub spot: f64,
    pub yield_curve: crate::hexagon::domain::treasury::YieldCurve,
    pub valuation_dates: Option<Vec<NaiveDate>>,
    pub strategy_kind: SimulationStrategyKind,
    pub volatility_shifts: Vec<f64>,
    pub legs: Vec<SimulationLegSelection>,
}

/// Technology-neutral input for constructing a scenario grid.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScenarioGridRequest {
    pub spot: f64,
    pub range_fraction: f64,
    pub spot_count: usize,
    pub valuation_dates: Vec<NaiveDate>,
    pub volatility_shifts: Vec<f64>,
    pub required_spots: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SimulationCatalogRequest {
    pub ticker: String,
    pub snapshot: Snapshot,
    pub spot: f64,
}

/// Provided interface containing the complete strategy-simulation conversation.
#[async_trait]
pub trait ForSimulatingStrategies: Send + Sync {
    async fn simulation_catalog(
        &self,
        request: SimulationCatalogRequest,
    ) -> PortResult<SimulationCatalog>;

    async fn build_scenario_grid(&self, request: ScenarioGridRequest) -> PortResult<ScenarioGrid>;

    async fn simulate_strategy(&self, request: SimulationRequest) -> PortResult<SimulationResult>;

    async fn simulate_scenario(&self, command: SimulateScenario) -> PortResult<SimulationScenario>;
}
