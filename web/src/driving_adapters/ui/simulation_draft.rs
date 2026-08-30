use crate::application::{asset_options::ContractDetail, asset_simulation::SimulationLeg};
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
        quantity: 1,
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
    let mut legs = read_draft_legs();
    if let Some(existing) = legs.iter_mut().find(|existing| existing.key == leg.key) {
        existing.quantity = existing.quantity.saturating_add(leg.quantity);
        existing.price = leg.price;
    } else {
        legs.push(leg);
    }
    legs.retain(|leg| leg.quantity != 0);
    write_draft_legs(&legs)
}

pub fn contains_draft_leg(key: &str) -> bool {
    read_draft_legs().iter().any(|leg| leg.key == key)
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
}
