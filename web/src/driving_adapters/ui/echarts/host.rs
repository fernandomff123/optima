use leptos::prelude::*;

use super::dispose_chart;

#[component]
pub fn EChartsHost(
    #[prop(into)] id: String,
    #[prop(into)] label: String,
    #[prop(into)] class: String,
) -> impl IntoView {
    let cleanup_id = id.clone();
    on_cleanup(move || dispose_chart(&cleanup_id));

    view! {
        <div id=id class=class role="img" aria-label=label></div>
    }
}
