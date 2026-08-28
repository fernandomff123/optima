use crate::{
    domain::navigation::GLOBAL_NAV,
    driving_adapters::ui::components::{ShellIcon, ShellIconKind},
};
use leptos::prelude::*;
use leptos_router::{components::A, hooks::use_location};

fn navigation_icon(href: &str) -> AnyView {
    let kind = match href {
        "/" => ShellIconKind::Dashboard,
        "/markets" => ShellIconKind::Markets,
        "/assets" => ShellIconKind::Search,
        "/options" => ShellIconKind::Options,
        "/volatility" => ShellIconKind::Volatility,
        "/gex" => ShellIconKind::Gex,
        "/simulations" => ShellIconKind::Simulations,
        "/portfolio" => ShellIconKind::Portfolio,
        "/settings" => ShellIconKind::Settings,
        _ => return ().into_any(),
    };
    view! { <ShellIcon kind=kind class="size-4 shrink-0" /> }.into_any()
}

#[component]
pub fn Sidebar() -> impl IntoView {
    let location = use_location();
    view! {
        <aside class="hidden min-h-screen w-20 shrink-0 flex-col border-r border-border bg-sidebar lg:flex xl:w-56">
            <a class="flex h-12 items-center gap-3 border-b border-border px-5" href="/" aria-label="Optima dashboard">
                <span class="grid size-7 place-items-center rounded bg-interactive-source text-xs font-black text-white xl:hidden">"O"</span>
                <span class="hidden text-sm font-bold tracking-[0.18em] text-text-primary xl:inline">"OPTIMA"</span>
            </a>
            <nav class="flex-1 space-y-1 px-3 py-5" aria-label="Primary navigation">
                {GLOBAL_NAV.into_iter().map(|item| {
                    let aria_location = location.clone();
                    let class_location = location.clone();
                    view! {
                        {item.separator_before.then(|| view! { <div class="my-3 border-t border-border" aria-hidden="true"></div> })}
                        <A href=item.href
                            attr:aria-label=item.label
                            attr:title=item.label
                            attr:aria-current=move || item.is_current(&aria_location.pathname.get()).then_some("page")
                            attr:class=move || if item.is_current(&class_location.pathname.get()) {
                                "group flex h-10 items-center gap-3 rounded border border-interactive-source/40 bg-state-selected px-3 text-xs font-semibold text-text-primary"
                            } else {
                                "group flex h-10 items-center gap-3 rounded border border-transparent px-3 text-xs font-medium text-text-secondary hover:bg-state-hover hover:text-text-primary"
                            }>
                            {navigation_icon(item.href)}
                            <span class="hidden xl:inline">{item.label}</span>
                        </A>
                    }
                }).collect_view()}
            </nav>
            <div class="border-t border-border p-4 text-[0.625rem] leading-4 text-text-muted-readable">
                <span class="hidden xl:block">"Visual foundation · no live data"</span>
                <span class="block text-center xl:hidden" aria-label="Visual foundation">"V2"</span>
            </div>
        </aside>
    }
}

#[component]
pub fn MobileNavigation() -> impl IntoView {
    let location = use_location();
    view! {
        <nav class="dense-scrollbar flex overflow-x-auto border-b border-border bg-sidebar px-2 lg:hidden" aria-label="Mobile primary navigation">
            {GLOBAL_NAV.into_iter().map(|item| {
                let aria_location = location.clone();
                let class_location = location.clone();
                view! {
                    {item.separator_before.then(|| view! { <span class="my-2 border-l border-border" aria-hidden="true"></span> })}
                    <A href=item.href
                        attr:aria-current=move || item.is_current(&aria_location.pathname.get()).then_some("page")
                        attr:class=move || if item.is_current(&class_location.pathname.get()) {
                            "shrink-0 border-b-2 border-interactive-text px-3 py-3 text-xs font-semibold text-text-primary"
                        } else {
                            "shrink-0 border-b-2 border-transparent px-3 py-3 text-xs font-medium text-text-secondary hover:text-text-primary"
                        }>{item.label}</A>
                }
            }).collect_view()}
        </nav>
    }
}
