use crate::application::asset_overview::ValueTone;
use leptos::prelude::*;

#[component]
pub fn FinancialValue(
    value: Option<String>,
    unit: Option<String>,
    tone: ValueTone,
    #[prop(into)] label: String,
    #[prop(optional)] compact_unit: bool,
) -> impl IntoView {
    let value = value.unwrap_or_else(|| "Unavailable".into());
    let accessible = unit.as_ref().map_or_else(
        || format!("{label}: {value}"),
        |unit| format!("{label}: {value} {unit}"),
    );
    let tone_class = match tone {
        ValueTone::Positive => "text-finance-positive",
        ValueTone::Negative => "text-negative-text",
        ValueTone::Neutral => "text-text-primary",
        ValueTone::Special => "text-level-special",
    };
    let layout = if compact_unit {
        "inline-grid w-full grid-cols-[minmax(0,1fr)_1.25rem] items-baseline"
    } else {
        "inline-grid w-[8.75rem] grid-cols-[minmax(0,1fr)_3.75rem] items-baseline"
    };
    view! {
        <span class=layout aria-label=accessible>
            <span class=format!("numeric text-right font-medium {tone_class}") aria-hidden="true">{value}</span>
            <span class="pl-1.5 text-left text-xs font-normal text-text-muted-readable" aria-hidden="true">{unit.unwrap_or_default()}</span>
        </span>
    }
}
