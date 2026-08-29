use crate::application::asset_overview::ValueTone;
use leptos::prelude::*;

#[component]
pub fn FinancialValue(
    value: Option<String>,
    #[prop(optional_no_strip)] suffix: Option<String>,
    unit: Option<String>,
    tone: ValueTone,
    #[prop(into)] label: String,
) -> impl IntoView {
    let value = value.unwrap_or_else(|| "Unavailable".into());
    let suffix = suffix.unwrap_or_default();
    let accessible = unit.as_ref().map_or_else(
        || format!("{label}: {value}{suffix}"),
        |unit| format!("{label}: {value}{suffix} {unit}"),
    );
    let tone_class = value_tone_class(tone);
    view! {
        <span class=format!("numeric inline-block whitespace-nowrap text-right font-medium {tone_class}") aria-label=accessible>
            {value}{suffix}
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
