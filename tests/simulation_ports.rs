use chrono::NaiveDate;
use hexagonal_backend::hexagon::{
    PortError,
    application::simulation::SimulationApplication,
    domain::{
        options::OptionType,
        simulation::{
            ExerciseStyle, MarketState, OptionContract, PricingConfig, PricingModel,
            SimulationRequest, Strategy, StrategyLeg,
        },
    },
    driving_ports::for_simulating_strategies::{ForSimulatingStrategies, ScenarioGridRequest},
};

fn date(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day).expect("valid test date")
}

#[tokio::test]
async fn driving_port_builds_grid_and_simulates_a_strategy() {
    let application = SimulationApplication;
    let grid = application
        .build_scenario_grid(ScenarioGridRequest {
            spot: 100.0,
            range_fraction: 0.20,
            spot_count: 5,
            valuation_dates: vec![date(2026, 1, 1)],
            volatility_shifts: vec![0.0],
            required_spots: vec![105.0],
        })
        .await
        .expect("valid grid must be built");
    assert!(grid.spots.contains(&105.0));

    let result = application
        .simulate_strategy(SimulationRequest {
            strategy: Strategy {
                id: Some("long-call".to_string()),
                root: "SPY".to_string(),
                legs: vec![StrategyLeg {
                    contract: OptionContract {
                        symbol: "SPY260201C00100000".to_string(),
                        option_type: OptionType::Call,
                        exercise_style: ExerciseStyle::European,
                        strike: 100.0,
                        expiration: date(2026, 2, 1),
                    },
                    quantity: 1,
                    multiplier: 100,
                    entry_price: 5.0,
                    entry_volatility: Some(0.20),
                    fees: 0.0,
                }],
            },
            market: MarketState {
                valuation_date: date(2026, 1, 1),
                spot: 100.0,
                risk_free_rate: 0.03,
                dividend_yield: 0.0,
                volatility: 0.20,
                snapshot_id: None,
            },
            grid,
            pricing: PricingConfig {
                european_model: PricingModel::BlackScholes,
                american_model: PricingModel::Binomial { steps: 100 },
            },
        })
        .await
        .expect("valid strategy must be simulated");

    assert_eq!(result.strategy_id.as_deref(), Some("long-call"));
    assert_eq!(result.points.len(), 6);
}

#[tokio::test]
async fn invalid_grid_is_reported_in_application_language() {
    let error = SimulationApplication
        .build_scenario_grid(ScenarioGridRequest {
            spot: 0.0,
            range_fraction: 0.20,
            spot_count: 5,
            valuation_dates: vec![date(2026, 1, 1)],
            volatility_shifts: vec![0.0],
            required_spots: Vec::new(),
        })
        .await
        .expect_err("zero spot must be rejected");

    assert!(matches!(error, PortError::InvalidRequest(_)));
}
