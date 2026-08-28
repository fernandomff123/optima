use crate::domain::{asset::AssetCapability, navigation::asset_tab_path};
use leptos::prelude::*;
use leptos_router::{components::A, hooks::use_location};

#[component]
pub fn AssetTabs(
    #[prop(into)] ticker: String,
    #[prop(optional)] capabilities: Vec<AssetCapability>,
) -> impl IntoView {
    let location = use_location();
    let capabilities = if capabilities.is_empty() {
        vec![
            AssetCapability::Overview,
            AssetCapability::Chart,
            AssetCapability::Options,
            AssetCapability::Volatility,
            AssetCapability::Gex,
            AssetCapability::Simulation,
        ]
    } else {
        capabilities
    };
    view! {
        <nav class="dense-scrollbar overflow-x-auto border-b border-border" aria-label="Asset workspace">
            <div class="flex min-w-max px-4 sm:px-6">
                {capabilities.into_iter().map(|capability| {
                    let label = capability.label();
                    let segment = capability.segment();
                    let href = asset_tab_path(&ticker, segment);
                    let aria_href = href.clone();
                    let class_href = href.clone();
                    let aria_location = location.clone();
                    let class_location = location.clone();
                    view! {
                        <A href=href
                            attr:aria-current=move || (aria_location.pathname.get() == aria_href).then_some("page")
                            attr:class=move || if class_location.pathname.get() == class_href {
                                "border-b-2 border-interactive-text px-4 py-3 text-xs font-semibold text-text-primary"
                            } else {
                                "border-b-2 border-transparent px-4 py-3 text-xs font-medium text-text-secondary hover:text-text-primary"
                            }>{label}</A>
                    }
                }).collect_view()}
            </div>
        </nav>
    }
}
