use crate::application::asset_overview::NewsItem;
use leptos::prelude::*;

#[component]
pub fn LatestNews(items: Vec<NewsItem>) -> impl IntoView {
    view! {
        <ul class="flex h-full flex-col">
            {items.into_iter().map(|item| view! {
                <li class="flex min-h-[4.75rem] flex-1 flex-col justify-center border-b border-border px-3 py-2 last:border-b-0">
                    <p class="text-sm font-medium leading-[1.25rem] text-text-primary">{item.headline}</p>
                    <div class="mt-1 flex justify-between gap-3 text-xs text-text-muted-readable"><span>{item.source}</span><span class="shrink-0">{item.age}</span></div>
                </li>
            }).collect_view()}
        </ul>
    }
}
