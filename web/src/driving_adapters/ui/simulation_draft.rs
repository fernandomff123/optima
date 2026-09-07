use crate::{
    application::{asset_options::ContractDetail, asset_simulation::SimulationLeg},
    ports::asset_simulation::{
        StrategySimulationLeg, StrategySimulationRequest, StrategySimulationSide,
    },
};
use serde_json::{Value, json};

#[derive(Clone, Debug, PartialEq)]
pub struct DraftLeg {
    pub key: String,
    pub quantity: i32,
    pub instrument: String,
    pub strike: String,
    pub expiration: String,
    pub price: String,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(inline_js = r#"
const OPTIMA_SIMULATION_DRAFT = "optima.simulation-draft.v1";

export function readOptimaSimulationDraft() {
  try { return localStorage.getItem(OPTIMA_SIMULATION_DRAFT) || "[]"; }
  catch (_) { return "[]"; }
}

export function writeOptimaSimulationDraft(value) {
  try { localStorage.setItem(OPTIMA_SIMULATION_DRAFT, value); return true; }
  catch (_) { return false; }
}
"#)]
extern "C" {
    #[wasm_bindgen::prelude::wasm_bindgen(js_name = readOptimaSimulationDraft)]
    fn read_storage() -> String;

    #[wasm_bindgen::prelude::wasm_bindgen(js_name = writeOptimaSimulationDraft)]
    fn write_storage(value: &str) -> bool;
}

#[cfg(not(target_arch = "wasm32"))]
fn read_storage() -> String {
    "[]".to_owned()
}

#[cfg(not(target_arch = "wasm32"))]
fn write_storage(_value: &str) -> bool {
    false
}

pub fn option_draft_leg(contract: &ContractDetail) -> DraftLeg {
    let strike = fact(contract, "Strike").unwrap_or("—");
    let expiration = fact(contract, "Expiration").unwrap_or("—");
    let instrument = fact(contract, "Type").unwrap_or("OPTION");
    DraftLeg {
        key: format!(
            "option:{}:{expiration}:{strike}:{instrument}",
            contract.title
        ),
        quantity: contract.quantity,
        instrument: instrument.to_uppercase(),
        strike: strike.to_owned(),
        expiration: expiration.to_owned(),
        price: contract.price.clone(),
    }
}

pub fn underlying_draft_leg(symbol: &str, quantity: i32) -> DraftLeg {
    DraftLeg {
        key: format!("underlying:{symbol}"),
        quantity,
        instrument: "STOCK".to_owned(),
        strike: "—".to_owned(),
        expiration: "—".to_owned(),
        price: "Market".to_owned(),
    }
}

pub fn base_draft_legs(legs: &[SimulationLeg]) -> Vec<DraftLeg> {
    legs.iter()
        .enumerate()
        .map(|(index, leg)| DraftLeg {
            key: format!("base:{index}:{}:{}", leg.option_type, leg.strike),
            quantity: leg.quantity,
            instrument: leg.option_type.clone(),
            strike: leg.strike.clone(),
            expiration: leg.expiration.clone(),
            price: leg.price.clone(),
        })
        .collect()
}

pub fn read_draft_legs() -> Vec<DraftLeg> {
    let Ok(Value::Array(values)) = serde_json::from_str::<Value>(&read_storage()) else {
        return Vec::new();
    };
    values.into_iter().filter_map(from_json).collect()
}

pub fn write_draft_legs(legs: &[DraftLeg]) -> bool {
    let values = legs.iter().map(to_json).collect::<Vec<_>>();
    write_storage(&Value::Array(values).to_string())
}

pub fn upsert_draft_leg(leg: DraftLeg) -> bool {
    upsert_draft_leg_with_quantity(leg).is_some()
}

pub fn upsert_draft_leg_with_quantity(leg: DraftLeg) -> Option<i32> {
    let key = leg.key.clone();
    let mut legs = read_draft_legs();
    if let Some(existing) = legs.iter_mut().find(|existing| existing.key == key) {
        existing.quantity = existing.quantity.saturating_add(leg.quantity);
        existing.price = leg.price;
    } else {
        legs.push(leg);
    }
    let quantity = legs
        .iter()
        .find(|existing| existing.key == key)
        .map(|existing| existing.quantity)
        .unwrap_or_default();
    legs.retain(|leg| leg.quantity != 0);
    write_draft_legs(&legs).then_some(quantity)
}

pub fn contains_draft_leg(key: &str) -> bool {
    read_draft_legs().iter().any(|leg| leg.key == key)
}

pub fn strategy_simulation_request(
    symbol: &str,
    legs: &[DraftLeg],
    valuation_date: &str,
    spot: f64,
    scenario_spot: f64,
    volatility: f64,
    volatility_shift: f64,
    time_days: f64,
) -> Result<StrategySimulationRequest, String> {
    if legs
        .iter()
        .any(|leg| leg.instrument.eq_ignore_ascii_case("STOCK"))
    {
        return Err("Underlying legs are not supported by the simulation API yet".to_owned());
    }
    let legs = legs
        .iter()
        .filter(|leg| {
            leg.instrument.eq_ignore_ascii_case("CALL")
                || leg.instrument.eq_ignore_ascii_case("PUT")
        })
        .map(|leg| {
            let entry_price = numeric(&leg.price)
                .ok_or_else(|| format!("Invalid entry price for {}", leg.key))?;
            let expiration = iso_expiration(&leg.expiration)
                .ok_or_else(|| format!("Invalid expiration for {}", leg.key))?;
            Ok(StrategySimulationLeg {
                option_type: if leg.instrument.eq_ignore_ascii_case("CALL") {
                    "Call".to_owned()
                } else {
                    "Put".to_owned()
                },
                strike: numeric(&leg.strike)
                    .ok_or_else(|| format!("Invalid strike for {}", leg.key))?,
                side: if leg.quantity < 0 {
                    StrategySimulationSide::Sell
                } else {
                    StrategySimulationSide::Buy
                },
                quantity: leg.quantity.unsigned_abs(),
                entry_price,
                expiration,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    if legs.is_empty() {
        return Err("No option legs selected".to_owned());
    }
    let valuation_date = iso_expiration(valuation_date)
        .ok_or_else(|| "Invalid simulation valuation date".to_owned())?;
    let analysis_date = add_days_iso(&valuation_date, time_days.round().max(0.0) as u32)
        .ok_or_else(|| "Invalid simulation analysis date".to_owned())?;
    Ok(StrategySimulationRequest {
        ticker: symbol.trim().to_ascii_uppercase(),
        valuation_date,
        analysis_date,
        spot,
        scenario_spot,
        volatility,
        volatility_shift,
        risk_free_rate: 0.0,
        dividend_yield: 0.0,
        legs,
    })
}

fn iso_expiration(value: &str) -> Option<String> {
    let parts = value.split_whitespace().collect::<Vec<_>>();
    if parts.len() < 3 {
        return None;
    }
    let year = parts[2].parse::<u16>().ok()?;
    let (day, month) = if let Ok(day) = parts[0].parse::<u8>() {
        (day, month_number(parts[1])?)
    } else {
        (
            parts[1].trim_end_matches(',').parse::<u8>().ok()?,
            month_number(parts[0])?,
        )
    };
    Some(format!("{year:04}-{month:02}-{day:02}"))
}

fn add_days_iso(value: &str, days: u32) -> Option<String> {
    let mut parts = value.split('-');
    let mut year = parts.next()?.parse::<u32>().ok()?;
    let mut month = parts.next()?.parse::<u32>().ok()?;
    let mut day = parts.next()?.parse::<u32>().ok()?;
    if parts.next().is_some() || !(1..=12).contains(&month) || day == 0 {
        return None;
    }
    for _ in 0..days {
        day += 1;
        if day > days_in_month(year, month) {
            day = 1;
            month += 1;
            if month > 12 {
                month = 1;
                year += 1;
            }
        }
    }
    Some(format!("{year:04}-{month:02}-{day:02}"))
}

fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        4 | 6 | 9 | 11 => 30,
        2 if year % 400 == 0 || (year % 4 == 0 && year % 100 != 0) => 29,
        2 => 28,
        _ => 31,
    }
}

fn month_number(value: &str) -> Option<u8> {
    match value.to_ascii_lowercase().as_str() {
        "jan" | "january" => Some(1),
        "feb" | "february" => Some(2),
        "mar" | "march" => Some(3),
        "apr" | "april" => Some(4),
        "may" => Some(5),
        "jun" | "june" => Some(6),
        "jul" | "july" => Some(7),
        "aug" | "august" => Some(8),
        "sep" | "sept" | "september" => Some(9),
        "oct" | "october" => Some(10),
        "nov" | "november" => Some(11),
        "dec" | "december" => Some(12),
        _ => None,
    }
}

fn numeric(value: &str) -> Option<f64> {
    value
        .trim()
        .trim_start_matches('$')
        .replace(',', "")
        .parse()
        .ok()
}

fn fact<'a>(contract: &'a ContractDetail, label: &str) -> Option<&'a str> {
    contract
        .facts
        .iter()
        .find(|(candidate, _)| candidate.eq_ignore_ascii_case(label))
        .map(|(_, value)| value.as_str())
}

fn to_json(leg: &DraftLeg) -> Value {
    json!({
        "key": leg.key,
        "quantity": leg.quantity,
        "instrument": leg.instrument,
        "strike": leg.strike,
        "expiration": leg.expiration,
        "price": leg.price,
    })
}

fn from_json(value: Value) -> Option<DraftLeg> {
    Some(DraftLeg {
        key: value.get("key")?.as_str()?.to_owned(),
        quantity: i32::try_from(value.get("quantity")?.as_i64()?).ok()?,
        instrument: value.get("instrument")?.as_str()?.to_owned(),
        strike: value.get("strike")?.as_str()?.to_owned(),
        expiration: value.get("expiration")?.as_str()?.to_owned(),
        price: value.get("price")?.as_str()?.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_draft_storage_is_safe_and_provider_neutral() {
        assert!(read_draft_legs().is_empty());
        assert!(!write_draft_legs(&[underlying_draft_leg("AAPL", 100)]));
    }

    #[test]
    fn draft_options_map_to_the_existing_simulation_contract() {
        let request = strategy_simulation_request(
            "AAPL",
            &[DraftLeg {
                key: "call".to_owned(),
                quantity: -2,
                instrument: "CALL".to_owned(),
                strike: "192.50".to_owned(),
                expiration: "17 May 2025".to_owned(),
                price: "2.61".to_owned(),
            }],
            "May 10, 2025",
            191.13,
            198.0,
            0.238,
            0.0,
            7.0,
        )
        .unwrap();
        assert_eq!(request.legs[0].option_type, "Call");
        assert_eq!(request.legs[0].strike, 192.5);
        assert_eq!(request.legs[0].side, StrategySimulationSide::Sell);
        assert_eq!(request.legs[0].quantity, 2);
        assert_eq!(request.valuation_date, "2025-05-10");
        assert_eq!(request.analysis_date, "2025-05-17");
    }

    #[test]
    fn valuation_date_accepts_the_mock_header_format() {
        assert_eq!(
            iso_expiration("May 10, 2025").as_deref(),
            Some("2025-05-10")
        );
    }

    #[test]
    fn selected_time_advances_the_analysis_date() {
        assert_eq!(add_days_iso("2025-05-30", 3).as_deref(), Some("2025-06-02"));
    }
}
