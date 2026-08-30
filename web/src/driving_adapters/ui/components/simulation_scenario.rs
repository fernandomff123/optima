use crate::application::asset_simulation::ScenarioControl;
use leptos::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScenarioSelection {
    pub spot: f64,
    pub implied_volatility: f64,
    pub time_days: f64,
}

impl ScenarioSelection {
    pub fn from_controls(controls: &[ScenarioControl]) -> Self {
        Self {
            spot: target(controls, "Spot", 191.13),
            implied_volatility: target(controls, "Implied Volatility", 23.8),
            time_days: target(controls, "Time", 0.0),
        }
    }
}

#[component]
pub fn SimulationScenarioPanel(
    preset: String,
    controls: Vec<ScenarioControl>,
    selection: RwSignal<ScenarioSelection>,
) -> impl IntoView {
    let initial = ScenarioSelection::from_controls(&controls);
    let (saved, set_saved) = signal(false);
    view! {
        <aside class="flex h-full min-h-0 flex-col border border-border bg-surface" aria-label="Mock scenario controls">
            <div class="panel-header"><h2 class="text-sm font-semibold">"Scenario"</h2><span class="text-[0.625rem] font-semibold uppercase tracking-wider text-level-special">"Interactive fixture"</span></div>
            <div class="dense-scrollbar min-h-0 flex-1 overflow-y-auto">
                <div class="border-b border-border p-4">
                    <label class="flex items-center gap-3 text-xs text-text-secondary"><span>"Preset"</span><select class="min-h-9 flex-1 rounded border border-border bg-canvas px-3 text-text-primary" disabled><option>{preset}</option></select></label>
                </div>
                {controls.into_iter().map(|control| view! {
                    <ScenarioSlider control selection />
                }).collect_view()}
                <button type="button" class="flex min-h-12 w-full items-center justify-between border-b border-border px-4 text-xs text-text-primary" disabled><span>"Advanced Settings"</span><span>"›"</span></button>
                <p class="px-4 py-3 text-[0.6875rem] leading-relaxed text-text-muted-readable">"Sliders select coordinates within the deterministic mock grid. The highlighted payoff marker and P&L heatmap cell update without browser-side pricing."</p>
            </div>
            <footer class="grid shrink-0 grid-cols-2 gap-3 border-t border-border p-4">
                <button type="button" class=move || if saved.get() { "min-h-10 rounded border border-interactive-source bg-state-selected px-3 text-xs font-semibold text-interactive-text" } else { "min-h-10 rounded border border-interactive-source px-3 text-xs font-semibold text-interactive-text hover:bg-state-hover" } aria-pressed=move || saved.get() on:click=move |_| set_saved.set(true)>{move || if saved.get() { "Scenario Saved" } else { "Save Scenario" }}</button>
                <button type="button" class="min-h-10 rounded border border-border px-3 text-xs font-semibold text-text-secondary hover:bg-state-hover hover:text-text-primary" on:click=move |_| { selection.set(initial); set_saved.set(false); }>"Reset"</button>
            </footer>
        </aside>
    }
}

#[component]
fn ScenarioSlider(
    control: ScenarioControl,
    selection: RwSignal<ScenarioSelection>,
) -> impl IntoView {
    let label = control.label.clone();
    let event_label = label.clone();
    let display_label = label.clone();
    let minimum = numeric(&control.minimum).unwrap_or(0.0);
    let maximum = numeric(&control.maximum).unwrap_or(100.0);
    let step = if label == "Spot" {
        0.5
    } else if label == "Time" {
        1.0
    } else {
        0.5
    };
    view! {
        <section class="border-b border-border p-4">
            <div class="flex items-center justify-between gap-3 text-sm">
                <h3 class="font-medium text-text-primary">{label.clone()}</h3>
                <p class="numeric text-interactive-text">{control.current}<span class="px-2 text-text-secondary">"→"</span>{move || format_value(&display_label, selected(selection, &display_label))}</p>
            </div>
            <input
                type="range"
                min=minimum.to_string()
                max=maximum.to_string()
                step=step.to_string()
                prop:value=move || selected(selection, &label).to_string()
                class="mt-4 h-2 w-full cursor-pointer accent-interactive-source"
                aria-label=format!("{} scenario value", control.label)
                on:input=move |event| {
                    if let Ok(value) = event_target_value(&event).parse::<f64>() {
                        selection.update(|state| set_selected(state, &event_label, value));
                    }
                }
            />
            <div class="mt-2 flex justify-between text-[0.6875rem] text-text-secondary numeric"><span>{control.minimum}</span><span>{control.maximum}</span></div>
        </section>
    }
}

fn selected(selection: RwSignal<ScenarioSelection>, label: &str) -> f64 {
    let state = selection.get();
    match label {
        "Spot" => state.spot,
        "Time" => state.time_days,
        _ => state.implied_volatility,
    }
}

fn set_selected(state: &mut ScenarioSelection, label: &str, value: f64) {
    match label {
        "Spot" => state.spot = value,
        "Time" => state.time_days = value,
        _ => state.implied_volatility = value,
    }
}

fn target(controls: &[ScenarioControl], label: &str, fallback: f64) -> f64 {
    controls
        .iter()
        .find(|control| control.label == label)
        .and_then(|control| numeric(&control.target))
        .unwrap_or(fallback)
}

fn numeric(value: &str) -> Option<f64> {
    let normalized = value
        .chars()
        .filter(|character| character.is_ascii_digit() || matches!(character, '.' | '-'))
        .collect::<String>();
    normalized.parse().ok()
}

fn format_value(label: &str, value: f64) -> String {
    match label {
        "Spot" => format!("{value:.2}"),
        "Time" if value == 0.0 => "Today".to_owned(),
        "Time" => format!("+{value:.0} days"),
        _ => format!("{value:.1}%"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scenario_values_are_parsed_without_financial_calculation() {
        assert_eq!(numeric("23.8%"), Some(23.8));
        assert_eq!(numeric("+7 days"), Some(7.0));
        assert_eq!(format_value("Time", 0.0), "Today");
    }
}
