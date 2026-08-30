use crate::application::asset_simulation::ScenarioControl;
use leptos::prelude::*;

#[component]
pub fn SimulationScenarioPanel(preset: String, controls: Vec<ScenarioControl>) -> impl IntoView {
    let (saved, set_saved) = signal(false);
    view! {
        <aside class="flex h-full min-h-0 flex-col border border-border bg-surface" aria-label="Mock scenario controls">
            <div class="panel-header"><h2 class="text-sm font-semibold">"Scenario"</h2><span class="text-[0.625rem] font-semibold uppercase tracking-wider text-level-special">"Fixture"</span></div>
            <div class="dense-scrollbar min-h-0 flex-1 overflow-y-auto">
                <div class="border-b border-border p-4">
                    <label class="flex items-center gap-3 text-xs text-text-secondary"><span>"Preset"</span><select class="min-h-9 flex-1 rounded border border-border bg-canvas px-3 text-text-primary" disabled><option>{preset}</option></select></label>
                </div>
                {controls.into_iter().map(|control| view! {
                    <section class="border-b border-border p-4">
                        <div class="flex items-center justify-between gap-3 text-sm"><h3 class="font-medium text-text-primary">{control.label}</h3><p class="numeric text-interactive-text">{control.current} <span class="px-2 text-text-secondary">"→"</span> {control.target}</p></div>
                        <div class="mt-4 h-1.5 rounded-full bg-border"><div class="h-full rounded-full bg-interactive-source" style=format!("width: {}%", control.position_percent)></div></div>
                        <div class="mt-2 flex justify-between text-[0.6875rem] text-text-secondary numeric"><span>{control.minimum}</span><span>{control.maximum}</span></div>
                    </section>
                }).collect_view()}
                <button type="button" class="flex min-h-12 w-full items-center justify-between border-b border-border px-4 text-xs text-text-primary" disabled><span>"Advanced Settings"</span><span>"›"</span></button>
                <p class="px-4 py-3 text-[0.6875rem] leading-relaxed text-text-muted-readable">"Scenario controls are illustrative. Results are fixed mock snapshots and are not repriced in the browser."</p>
            </div>
            <footer class="grid shrink-0 grid-cols-2 gap-3 border-t border-border p-4">
                <button type="button" class=move || if saved.get() { "min-h-10 rounded border border-interactive-source bg-state-selected px-3 text-xs font-semibold text-interactive-text" } else { "min-h-10 rounded border border-interactive-source px-3 text-xs font-semibold text-interactive-text hover:bg-state-hover" } aria-pressed=move || saved.get() on:click=move |_| set_saved.set(true)>{move || if saved.get() { "Scenario Saved" } else { "Save Scenario" }}</button>
                <button type="button" class="min-h-10 rounded border border-border px-3 text-xs font-semibold text-text-secondary hover:bg-state-hover hover:text-text-primary" on:click=move |_| set_saved.set(false)>"Reset"</button>
            </footer>
        </aside>
    }
}
