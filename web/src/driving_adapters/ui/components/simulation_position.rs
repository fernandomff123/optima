use super::{ShellIcon, ShellIconKind};
use crate::driving_adapters::ui::simulation_draft::{DraftLeg, write_draft_legs};
use leptos::prelude::*;

#[component]
pub fn SimulationPosition(
    symbol: String,
    strategy_name: String,
    rows: RwSignal<Vec<DraftLeg>>,
    editor_open: Signal<bool>,
    on_add_option: Callback<()>,
    on_add_underlying: Callback<()>,
    on_done: Callback<()>,
) -> impl IntoView {
    let _ = strategy_name;
    let shares_label = format!("＋ Add {symbol} shares");
    view! {
        <section class="relative flex h-full min-h-0 flex-col overflow-hidden border border-border bg-surface" aria-label="Simulation strategy builder">
            <div class="panel-header"><h2 class="text-sm font-semibold uppercase tracking-wide">"Strategy Builder"</h2><span class="text-[0.625rem] font-semibold uppercase tracking-wider text-level-special">"Browser draft"</span></div>
            <div class="flex items-center gap-2 px-3 py-2"><label class="sr-only" for="simulation-strategy">"Strategy"</label><select id="simulation-strategy" class="min-h-9 min-w-0 flex-1 rounded border border-border bg-canvas px-3 text-xs text-text-primary" disabled><option>{move || detected_strategy(&rows.get())}</option></select><button type="button" class="shrink-0 text-[0.6875rem] font-medium text-interactive-text" disabled>"Save as template"</button></div>
            <div class="flex items-center justify-between border-y border-border bg-canvas px-3 py-2 text-[0.6875rem]"><span class="text-text-secondary">"Legs"</span><span class="numeric font-semibold text-text-primary">{move || rows.get().len()}</span></div>
            <div class="dense-scrollbar min-h-[4rem] flex-1 overflow-auto overscroll-contain" style="scrollbar-gutter: stable;">
                <table class="w-full min-w-[23rem] table-fixed text-[0.6875rem]">
                    <thead class="sticky top-0 z-10 border-b border-border bg-surface text-text-secondary"><tr><th class="w-10 px-2 py-2 text-left font-medium">"Leg"</th><th class="w-12 px-1 py-2 text-left font-medium">"Type"</th><th class="w-12 px-1 py-2 text-right font-medium">"Strike"</th><th class="w-16 px-1 py-2 text-left font-medium">"Exp"</th><th class="w-14 px-1 py-2 text-right font-medium">"Qty"</th><th class="w-14 px-1 py-2 text-right font-medium">"Price"</th><th class="w-7 px-1 py-2"><span class="sr-only">"Actions"</span></th></tr></thead>
                    <tbody class="divide-y divide-border numeric">
                        {move || rows.get().into_iter().map(|leg| {
                            let quantity = leg.quantity;
                            let positive = quantity > 0;
                            let instrument = leg.instrument;
                            let strike = leg.strike;
                            let expiration = compact_expiration(&leg.expiration);
                            let price = leg.price;
                            let increase_key = leg.key.clone();
                            let decrease_key = leg.key.clone();
                            let remove_key = leg.key.clone();
                            view! { <tr class="hover:bg-state-hover"><td class=if positive { "px-2 py-2.5 font-semibold text-finance-positive" } else { "px-2 py-2.5 font-semibold text-negative-text" }>{if positive { "+1" } else { "-1" }}</td><td class=if positive { "px-1 py-2.5 font-semibold text-finance-positive" } else { "px-1 py-2.5 font-semibold text-negative-text" }>{instrument}</td><td class="px-1 py-2.5 text-right text-text-primary">{strike}</td><td class="px-1 py-2.5 leading-tight text-text-primary">{expiration}</td><td class="px-1 py-2 text-right text-text-primary"><span class="inline-flex items-center justify-end gap-1"><span class="min-w-4 text-right">{quantity.unsigned_abs()}</span><span class="inline-flex flex-col gap-px"><button type="button" class="flex h-3.5 w-4 items-center justify-center rounded-sm border border-border text-[0.625rem] leading-none text-text-secondary hover:bg-state-hover" aria-label="Increase quantity" on:click=move |_| adjust_quantity(rows, &increase_key, 1)>"+"</button><button type="button" class="flex h-3.5 w-4 items-center justify-center rounded-sm border border-border text-[0.625rem] leading-none text-text-secondary hover:bg-state-hover" aria-label="Decrease quantity" on:click=move |_| adjust_quantity(rows, &decrease_key, -1)>"−"</button></span></span></td><td class="px-1 py-2.5 text-right text-text-primary">{price}</td><td class="px-1 py-2 text-center"><button type="button" class="inline-flex size-5 items-center justify-center rounded text-text-secondary hover:bg-negative-bg hover:text-negative-text" aria-label="Remove leg" on:click=move |_| remove_leg(rows, &remove_key)><ShellIcon kind=ShellIconKind::Trash class="size-3" /></button></td></tr> }
                        }).collect_view()}
                    </tbody>
                </table>
            </div>
            {move || (!editor_open.get()).then(|| view! { <div class="grid shrink-0 gap-1.5 border-t border-border px-3 py-2"><button type="button" class="min-h-8 rounded border border-interactive-source text-xs font-semibold text-interactive-text hover:bg-state-hover" on:click=move |_| on_add_option.run(())>"＋ Add option leg"</button><button type="button" class="min-h-8 rounded border border-interactive-source text-xs font-semibold text-interactive-text hover:bg-state-hover" on:click=move |_| on_add_underlying.run(())>{shares_label.clone()}</button></div> })}
            {move || if rows.get().is_empty() {
                view! { <div class="m-3 shrink-0 border border-border bg-canvas p-3 text-center text-[0.6875rem] text-text-secondary">"No legs selected"</div> }.into_any()
            } else {
                let summary = draft_summary(&rows.get());
                view! { <div class="mx-3 my-2 grid shrink-0 grid-cols-2 gap-x-4 gap-y-1.5 border border-border bg-canvas px-3 py-2 text-[0.6875rem]"><span class="text-text-secondary">{summary.net_label}</span><span class=summary.net_class>{summary.net_value}</span><span class="text-text-secondary">"Buying Power"</span><span class="numeric text-right text-text-primary">{summary.buying_power}</span><span class="text-text-secondary">"Max Risk"</span><span class="numeric text-right text-text-primary">{summary.max_risk}</span><span class="text-text-secondary">"Legs selected"</span><span class="numeric text-right text-text-primary">{summary.legs}</span></div> }.into_any()
            }}
            {move || editor_open.get().then(|| view! { <footer class="flex min-h-11 shrink-0 items-center justify-between border-t border-border px-4 text-xs"><span class="inline-flex items-center gap-2 text-text-secondary"><span class="size-2 rounded-full bg-interactive-text"></span>"Adding option leg…"</span><button type="button" class="rounded border border-interactive-source px-3 py-1.5 font-semibold text-interactive-text hover:bg-state-hover" on:click=move |_| on_done.run(())>"Done editing"</button></footer> })}
        </section>
    }
}

struct DraftSummary {
    net_label: &'static str,
    net_value: String,
    net_class: &'static str,
    buying_power: String,
    max_risk: String,
    legs: usize,
}

fn draft_summary(legs: &[DraftLeg]) -> DraftSummary {
    let balance = legs.iter().map(cash_flow).sum::<f64>();
    let exposure = legs.iter().map(|leg| cash_flow(leg).abs()).sum::<f64>();
    DraftSummary {
        net_label: if balance >= 0.0 {
            "Net Credit"
        } else {
            "Net Debit"
        },
        net_value: format!("{:.0}", balance.abs()),
        net_class: if balance >= 0.0 {
            "numeric text-right font-semibold text-finance-positive"
        } else {
            "numeric text-right font-semibold text-negative-text"
        },
        buying_power: format!("{:.0}", exposure),
        max_risk: format!("{:.0}", exposure),
        legs: legs.len(),
    }
}

fn cash_flow(leg: &DraftLeg) -> f64 {
    let normalized = leg.price.trim().trim_start_matches('$').replace(',', "");
    let Ok(unit_price) = normalized.parse::<f64>() else {
        return 0.0;
    };
    let multiplier = if leg.instrument.eq_ignore_ascii_case("CALL")
        || leg.instrument.eq_ignore_ascii_case("PUT")
    {
        100.0
    } else {
        1.0
    };
    -(leg.quantity as f64) * unit_price * multiplier
}

fn detected_strategy(legs: &[DraftLeg]) -> String {
    let options = legs
        .iter()
        .filter(|leg| {
            leg.instrument.eq_ignore_ascii_case("CALL")
                || leg.instrument.eq_ignore_ascii_case("PUT")
        })
        .collect::<Vec<_>>();
    if options.is_empty() {
        return if legs
            .iter()
            .any(|leg| leg.instrument.eq_ignore_ascii_case("STOCK"))
        {
            "Underlying position".to_owned()
        } else {
            "No strategy".to_owned()
        };
    }
    if options.len() != 2
        || !options[0]
            .instrument
            .eq_ignore_ascii_case(&options[1].instrument)
        || options[0].strike != options[1].strike
        || options[0].quantity.signum() == options[1].quantity.signum()
    {
        return "Custom strategy".to_owned();
    }
    let Some(first_date) = expiration_key(&options[0].expiration) else {
        return "Custom strategy".to_owned();
    };
    let Some(second_date) = expiration_key(&options[1].expiration) else {
        return "Custom strategy".to_owned();
    };
    if first_date == second_date {
        return "Custom strategy".to_owned();
    }
    let (near, far) = if first_date < second_date {
        (options[0], options[1])
    } else {
        (options[1], options[0])
    };
    let side = if near.instrument.eq_ignore_ascii_case("CALL") {
        "Call"
    } else {
        "Put"
    };
    if near.quantity > 0 && far.quantity < 0 {
        format!("Short {side} Calendar")
    } else if near.quantity < 0 && far.quantity > 0 {
        format!("Long {side} Calendar")
    } else {
        "Custom strategy".to_owned()
    }
}

fn expiration_key(value: &str) -> Option<(i32, u8, u8)> {
    let parts = value.split_whitespace().collect::<Vec<_>>();
    if parts.len() < 3 {
        return None;
    }
    let year = parts[2].parse().ok()?;
    let (day, month) = if let Ok(day) = parts[0].parse() {
        (day, month_number(parts[1])?)
    } else {
        (parts[1].parse().ok()?, month_number(parts[0])?)
    };
    Some((year, month, day))
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

fn compact_expiration(value: &str) -> String {
    let parts = value.split_whitespace().collect::<Vec<_>>();
    match parts.as_slice() {
        [day, month, ..] => format!("{} {}", month.to_uppercase(), day),
        _ => value.to_owned(),
    }
}
fn adjust_quantity(rows: RwSignal<Vec<DraftLeg>>, key: &str, delta: i32) {
    rows.update(|legs| {
        if let Some(index) = legs.iter().position(|leg| leg.key == key) {
            let sign = if legs[index].quantity < 0 { -1 } else { 1 };
            let magnitude = legs[index].quantity.unsigned_abs() as i32;
            let next = (magnitude + delta).max(0);
            if next == 0 {
                legs.remove(index);
            } else {
                legs[index].quantity = sign * next;
            }
        }
        write_draft_legs(legs);
    });
}
fn remove_leg(rows: RwSignal<Vec<DraftLeg>>, key: &str) {
    rows.update(|legs| {
        legs.retain(|leg| leg.key != key);
        write_draft_legs(legs);
    });
}
