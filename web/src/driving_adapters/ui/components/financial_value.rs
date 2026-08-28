use crate::application::asset_overview::ValueTone;
use leptos::prelude::*;

#[component]
pub fn FinancialValue(
    value: Option<String>,
    #[prop(optional_no_strip)] suffix: Option<String>,
    unit: Option<String>,
    tone: ValueTone,
    #[prop(into)] label: String,
    #[prop(optional)] compact_unit: bool,
) -> impl IntoView {
    let value = value.unwrap_or_else(|| "Unavailable".into());
    let suffix = suffix.unwrap_or_default();
    let accessible = unit.as_ref().map_or_else(
        || format!("{label}: {value}{suffix}"),
        |unit| format!("{label}: {value}{suffix} {unit}"),
    );
    let tone_class = value_tone_class(tone);
    let layout = if compact_unit && unit.is_none() {
        "inline-block w-full"
    } else if compact_unit {
        "inline-grid w-full grid-cols-[minmax(0,1fr)_1.25rem] items-baseline"
    } else {
        "inline-grid w-[8.75rem] grid-cols-[minmax(0,1fr)_3.75rem] items-baseline"
    };
    view! {
        <span class=layout aria-label=accessible>
            <span class=format!("numeric whitespace-nowrap text-right font-medium {tone_class}") aria-hidden="true"><span>{value}</span><span>{suffix}</span></span>
            <span class="pl-1.5 text-left text-xs font-normal text-text-muted-readable" aria-hidden="true">{unit.unwrap_or_default()}</span>
        </span>
    }
}

pub(crate) fn value_tone_class(tone: ValueTone) -> &'static str {
    match tone {
        ValueTone::Positive => "text-finance-positive",
        ValueTone::Negative => "text-negative-text",
        ValueTone::Neutral => "text-text-primary",
        ValueTone::Special => "text-level-special",
    }
}
