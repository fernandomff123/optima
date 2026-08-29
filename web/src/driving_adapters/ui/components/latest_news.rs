use crate::application::asset_overview::NewsItem;
use leptos::prelude::*;

#[component]
pub fn LatestNews(items: Vec<NewsItem>) -> impl IntoView {
    view! {
        <div>
            <ul>
                {items.into_iter().map(|item| view! {
                    <li class="flex h-17 flex-col justify-center border-b border-border px-3 py-2">
                        <p class="line-clamp-2 text-sm font-medium leading-[1.15rem] text-text-primary">{item.headline}</p>
                        <div class="mt-1 flex justify-between gap-3 text-xs text-text-muted-readable"><span>{item.source}</span><span class="shrink-0">{item.age}</span></div>
                    </li>
                }).collect_view()}
            </ul>
            <button class="h-10 shrink-0 cursor-not-allowed border-t border-border px-3 text-left text-sm font-medium text-interactive-text opacity-80" type="button" disabled aria-label="View all news unavailable in this mock" title="View all news unavailable in this mock">"View All News ›"</button>
        </div>
    }
}
