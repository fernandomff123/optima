use crate::ports::asset_simulation::{
    StrategySimulationCurve, StrategySimulationFailure, StrategySimulationFuture,
    StrategySimulationGreeks, StrategySimulationLeg, StrategySimulationPoint,
    StrategySimulationPort, StrategySimulationRequest, StrategySimulationResult,
    StrategySimulationSide,
};
use serde_json::{Value, json};

#[derive(Clone, Copy, Debug, Default)]
pub struct HttpStrategySimulationAdapter;

impl StrategySimulationPort for HttpStrategySimulationAdapter {
    fn simulate(&self, request: StrategySimulationRequest) -> StrategySimulationFuture {
        Box::pin(async move {
            if request.legs.is_empty() {
                return Err(StrategySimulationFailure::Unsupported(
                    "No option legs selected".to_owned(),
                ));
            }
            let expiration = request
                .legs
                .iter()
                .map(|leg| leg.expiration.as_str())
                .min()
                .ok_or_else(|| {
                    StrategySimulationFailure::Unsupported(
                        "The option expiration is unavailable".to_owned(),
                    )
                })?
                .to_owned();
            let legs = request
                .legs
                .iter()
                .map(|leg| request_leg(&request.ticker, leg))
                .collect::<Vec<_>>();
            let valuation_dates = if request.analysis_date == expiration {
                vec![request.analysis_date.clone()]
            } else {
                vec![request.analysis_date.clone(), expiration.clone()]
            };
            let payload = json!({
                "grid_request": {
                    "spot": request.spot,
                    "range_fraction": 0.20,
                    "spot_count": 321,
                    "valuation_dates": valuation_dates,
                    "volatility_shifts": [request.volatility_shift],
                    "required_spots": [request.spot, request.scenario_spot]
                },
                "simulation": {
                    "strategy": {
                        "id": null,
                        "root": request.ticker,
                        "legs": legs
                    },
                    "market": {
                        "valuation_date": request.valuation_date,
                        "spot": request.spot,
                        "risk_free_rate": request.risk_free_rate,
                        "dividend_yield": request.dividend_yield,
                        "volatility": request.volatility,
                        "snapshot_id": null
                    },
                    "pricing": {
                        "european_model": "BlackScholes",
                        "american_model": "BlackScholes"
                    }
                }
            });
            let response = fetch_simulation(&payload.to_string()).await?;
            parse_response(&response, &request, &expiration)
        })
    }
}

fn request_leg(ticker: &str, leg: &StrategySimulationLeg) -> Value {
    let signed_quantity = match leg.side {
        StrategySimulationSide::Buy => leg.quantity as i32,
        StrategySimulationSide::Sell => -(leg.quantity as i32),
    };
    json!({
        "contract": {
            "symbol": ticker,
            "option_type": leg.option_type,
            "exercise_style": "European",
            "strike": leg.strike,
            "expiration": leg.expiration
        },
        "quantity": signed_quantity,
        "multiplier": 100,
        "entry_price": leg.entry_price,
        "entry_volatility": null,
        "fees": 0.0
    })
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(inline_js = r#"
export async function requestOptimaStrategySimulation(payload) {
  const command = JSON.parse(payload);
  const gridResponse = await fetch("/api/strategy-simulation/grid", {
    method: "POST",
    headers: { "Accept": "application/json", "Content-Type": "application/json" },
    body: JSON.stringify(command.grid_request)
  });
  if (!gridResponse.ok) {
    const detail = await gridResponse.text();
    throw new Error(`scenario grid failed (${gridResponse.status})${detail ? `: ${detail}` : ""}`);
  }
  command.simulation.grid = await gridResponse.json();
  const response = await fetch("/api/strategy-simulation", {
    method: "POST",
    headers: { "Accept": "application/json", "Content-Type": "application/json" },
    body: JSON.stringify(command.simulation)
  });
  if (!response.ok) {
    const detail = await response.text();
    throw new Error(`strategy simulation failed (${response.status})${detail ? `: ${detail}` : ""}`);
  }
  return await response.text();
}
"#)]
extern "C" {
    #[wasm_bindgen::prelude::wasm_bindgen(catch, js_name = requestOptimaStrategySimulation)]
    async fn request_simulation(
        payload: &str,
    ) -> Result<wasm_bindgen::JsValue, wasm_bindgen::JsValue>;
}

#[cfg(target_arch = "wasm32")]
async fn fetch_simulation(payload: &str) -> Result<String, StrategySimulationFailure> {
    request_simulation(payload)
        .await
        .map_err(|error| {
            StrategySimulationFailure::Transport(
                error
                    .as_string()
                    .unwrap_or_else(|| "The simulation request failed".to_owned()),
            )
        })?
        .as_string()
        .ok_or_else(|| {
            StrategySimulationFailure::InvalidResponse(
                "The simulation response was not text".to_owned(),
            )
        })
}

#[cfg(not(target_arch = "wasm32"))]
async fn fetch_simulation(_payload: &str) -> Result<String, StrategySimulationFailure> {
    Err(StrategySimulationFailure::Transport(
        "HTTP simulation is available only in the browser".to_owned(),
    ))
}

fn parse_response(
    payload: &str,
    request: &StrategySimulationRequest,
    expiration: &str,
) -> Result<StrategySimulationResult, StrategySimulationFailure> {
    let value: Value = serde_json::from_str(payload).map_err(|error| {
        StrategySimulationFailure::InvalidResponse(format!("Invalid simulation JSON: {error}"))
    })?;
    let mut curves: Vec<StrategySimulationCurve> = Vec::new();
    for value in array(&value, "points")? {
        let valuation_date = string(value, "valuation_date")?;
        let volatility_shift = number(value, "volatility_shift")?;
        let point = parse_point(value)?;
        if let Some(curve) = curves.iter_mut().find(|curve| {
            curve.valuation_date == valuation_date
                && (curve.volatility_shift - volatility_shift).abs() < f64::EPSILON
        }) {
            curve.points.push(point);
        } else {
            curves.push(StrategySimulationCurve {
                label: format!("{valuation_date} · IV {:+.1}%", volatility_shift * 100.0),
                valuation_date,
                volatility_shift,
                points: vec![point],
            });
        }
    }
    for curve in &mut curves {
        curve
            .points
            .sort_by(|left, right| left.spot.total_cmp(&right.spot));
    }
    Ok(StrategySimulationResult {
        valuation_date: request.analysis_date.clone(),
        expiration: expiration.to_owned(),
        spot: request.spot,
        break_even_points: Vec::new(),
        curves,
    })
}

fn parse_point(value: &Value) -> Result<StrategySimulationPoint, StrategySimulationFailure> {
    let greeks = value.get("greeks").ok_or_else(|| invalid("greeks"))?;
    Ok(StrategySimulationPoint {
        spot: number(value, "spot")?,
        pnl: number(value, "pnl")?,
        greeks: StrategySimulationGreeks {
            delta: number(greeks, "delta")?,
            gamma: number(greeks, "gamma")?,
            theta: number(greeks, "theta")?,
            vega: number(greeks, "vega")?,
            rho: number(greeks, "rho")?,
        },
    })
}

fn string(value: &Value, field: &str) -> Result<String, StrategySimulationFailure> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| invalid(field))
}

fn number(value: &Value, field: &str) -> Result<f64, StrategySimulationFailure> {
    value
        .get(field)
        .and_then(Value::as_f64)
        .ok_or_else(|| invalid(field))
}

fn array<'a>(value: &'a Value, field: &str) -> Result<&'a Vec<Value>, StrategySimulationFailure> {
    value
        .get(field)
        .and_then(Value::as_array)
        .ok_or_else(|| invalid(field))
}

fn invalid(field: &str) -> StrategySimulationFailure {
    StrategySimulationFailure::InvalidResponse(format!(
        "Missing or invalid simulation field: {field}"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> StrategySimulationRequest {
        StrategySimulationRequest {
            ticker: "AAPL".to_owned(),
            valuation_date: "2025-05-10".to_owned(),
            analysis_date: "2025-05-14".to_owned(),
            spot: 191.13,
            scenario_spot: 198.0,
            volatility: 0.238,
            volatility_shift: 0.0,
            risk_free_rate: 0.0,
            dividend_yield: 0.0,
            legs: Vec::new(),
        }
    }

    #[test]
    fn response_parser_groups_canonical_backend_points() {
        let response = parse_response(
            r#"{"strategy_id":"AAPL","model":"BlackScholes","points":[{"spot":190.0,"valuation_date":"2025-05-17","volatility_shift":0.0,"theoretical_value":42.0,"pnl":42.0,"greeks":{"delta":0.1,"gamma":0.2,"theta":-0.3,"vega":0.4,"rho":0.5},"legs":[],"warnings":[]}]}"#,
            &request(),
            "2025-05-17",
        )
        .unwrap();
        assert_eq!(response.curves[0].points[0].pnl, 42.0);
        assert_eq!(response.expiration, "2025-05-17");
    }
}
