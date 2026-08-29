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
    let (collapsed, set_collapsed) = signal(false);
    let spx_selected = Signal::derive({
        let location = location.clone();
        move || location.pathname.get().starts_with("/assets/SPX/")
    });
    let aapl_selected = Signal::derive({
        let location = location.clone();
        move || location.pathname.get().starts_with("/assets/AAPL/")
    });
    view! {
        <aside class=move || if collapsed.get() { "hidden min-h-screen w-16 shrink-0 flex-col border-r border-border bg-sidebar lg:flex" } else { "hidden min-h-screen w-56 shrink-0 flex-col border-r border-border bg-sidebar lg:flex" }>
            <div class="flex h-14 shrink-0 items-center justify-between border-b border-border px-4">
                {move || (!collapsed.get()).then(|| view! { <a class="text-lg font-bold tracking-[0.12em] text-text-primary" href="/" aria-label="Optima dashboard">"OPTIMA"</a> })}
                <button class="grid size-8 place-items-center text-text-secondary hover:text-text-primary" type="button" aria-label=move || if collapsed.get() { "Expand sidebar" } else { "Compact sidebar" } title=move || if collapsed.get() { "Expand sidebar" } else { "Compact sidebar" } on:click=move |_| set_collapsed.update(|value| *value = !*value)>
                    <ShellIcon kind=ShellIconKind::Menu class="size-5" />
                </button>
            </div>
            <nav class="space-y-1 px-2 py-4" aria-label="Primary navigation">
                {GLOBAL_NAV.into_iter().filter(|item| item.href != "/settings").map(|item| {
                    let aria_location = location.clone();
                    let class_location = location.clone();
                    view! {
                        {item.separator_before.then(|| view! { <div class="my-3 border-t border-border" aria-hidden="true"></div> })}
                        <A href=item.href
                            attr:aria-label=item.label
                            attr:title=item.label
                            attr:aria-current=move || item.is_current(&aria_location.pathname.get()).then_some("page")
                            attr:class=move || if item.is_current(&class_location.pathname.get()) {
                                "group -mx-2 flex h-10 items-center gap-3 border-l-2 border-interactive-text bg-state-selected/40 px-5 text-sm font-medium text-interactive-text"
                            } else {
                                "group flex h-10 items-center gap-3 border-l-2 border-transparent px-3 text-sm font-medium text-text-secondary hover:bg-state-hover hover:text-text-primary"
                            }>
                            {navigation_icon(item.href)}
                            {move || (!collapsed.get()).then_some(item.label)}
                        </A>
                    }
                }).collect_view()}
            </nav>
            <div class="mx-5 border-t border-border" aria-hidden="true"></div>
            <div class="flex-1 px-2 py-3 numeric">
                <WatchlistRow symbol="SPX" value="5,303.27" change="+0.72%" positive=true href="/assets/SPX/overview" selected=spx_selected collapsed />
                <WatchlistRow symbol="NDX" value="18,404.48" change="+0.84%" positive=true selected=false collapsed />
                <WatchlistRow symbol="AAPL" value="191.13" change="+1.24%" positive=true href="/assets/AAPL/overview" selected=aapl_selected collapsed />
                <WatchlistRow symbol="NVDA" value="949.50" change="-0.51%" positive=false selected=false collapsed />
            </div>
            <div class="border-t border-border p-2">
                <button class=move || if collapsed.get() { "flex h-9 w-full items-center justify-center text-text-secondary hover:bg-state-hover hover:text-text-primary" } else { "flex h-9 w-full items-center gap-3 px-3 text-sm text-text-secondary hover:bg-state-hover hover:text-text-primary" } type="button" aria-label=move || if collapsed.get() { "Expand sidebar" } else { "Compact sidebar" } title=move || if collapsed.get() { "Expand sidebar" } else { "Compact sidebar" } on:click=move |_| set_collapsed.update(|value| *value = !*value)>
                    <span class=move || if collapsed.get() { "rotate-180" } else { "" }><ShellIcon kind=ShellIconKind::Collapse class="size-4" /></span>
                    {move || (!collapsed.get()).then_some("Compact sidebar")}
                </button>
            </div>
        </aside>
    }
}

#[component]
fn WatchlistRow(
    #[prop(into)] symbol: &'static str,
    #[prop(into)] value: &'static str,
    #[prop(into)] change: &'static str,
    positive: bool,
    #[prop(optional)] href: Option<&'static str>,
    #[prop(into)] selected: Signal<bool>,
    collapsed: ReadSignal<bool>,
) -> impl IntoView {
    let content = move || {
        view! {
            <span class="text-sm text-text-primary">{symbol}</span>
            {(!collapsed.get()).then(|| view! {
                <span class="ml-auto text-right text-sm leading-5">
                    <span class="block text-text-secondary">{value}</span>
                    <span class=if positive { "block text-finance-positive" } else { "block text-negative-text" }>{change}</span>
                </span>
            })}
        }
    };
    match href {
        Some(href) => view! { <A href=href attr:class=move || if selected.get() { "flex min-h-14 items-center border-l-2 border-interactive-text bg-state-selected/30 px-3" } else { "flex min-h-14 items-center border-l-2 border-transparent px-3" } attr:aria-current=move || selected.get().then_some("page") attr:title=symbol>{content}</A> }.into_any(),
        None => view! { <div class="flex min-h-14 cursor-not-allowed items-center border-l-2 border-transparent px-3 opacity-75" aria-disabled="true" title=format!("{symbol} overview unavailable")>{content}</div> }.into_any(),
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
