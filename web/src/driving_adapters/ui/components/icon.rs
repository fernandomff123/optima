//! Shell icons copied from the official Lucide icon set.
//! Source details are recorded in the frontend README.
//! License: ISC (Lucide); Search is derived from Feather under MIT.

use leptos::prelude::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShellIconKind {
    Dashboard,
    Markets,
    Search,
    Options,
    Volatility,
    Gex,
    Simulations,
    Portfolio,
    Settings,
    Menu,
    Bell,
    Launcher,
    Star,
    More,
    Collapse,
}

#[component]
pub fn ShellIcon(kind: ShellIconKind, #[prop(optional, into)] class: String) -> impl IntoView {
    let elements = match kind {
        ShellIconKind::Dashboard => {
            r#"<rect width="7" height="9" x="3" y="3" rx="1"/><rect width="7" height="5" x="14" y="3" rx="1"/><rect width="7" height="9" x="14" y="12" rx="1"/><rect width="7" height="5" x="3" y="16" rx="1"/>"#
        }
        ShellIconKind::Markets => {
            r#"<path d="M9 5v4"/><rect width="4" height="6" x="7" y="9" rx="1"/><path d="M9 15v2"/><path d="M17 3v2"/><rect width="4" height="8" x="15" y="5" rx="1"/><path d="M17 13v3"/><path d="M3 3v16a2 2 0 0 0 2 2h16"/>"#
        }
        ShellIconKind::Search => r#"<path d="m21 21-4.34-4.34"/><circle cx="11" cy="11" r="8"/>"#,
        ShellIconKind::Options => {
            r#"<circle cx="12" cy="12" r="10"/><path d="M16 8h-6a2 2 0 1 0 0 4h4a2 2 0 1 1 0 4H8"/><path d="M12 18V6"/>"#
        }
        ShellIconKind::Volatility => r#"<path d="M22 12h-4l-3 9L9 3l-3 9H2"/>"#,
        ShellIconKind::Gex => {
            r#"<line x1="18" x2="18" y1="20" y2="10"/><line x1="12" x2="12" y1="20" y2="4"/><line x1="6" x2="6" y1="20" y2="14"/>"#
        }
        ShellIconKind::Simulations => {
            r#"<line x1="6" x2="6" y1="3" y2="15"/><circle cx="18" cy="6" r="3"/><circle cx="6" cy="18" r="3"/><path d="M18 9a9 9 0 0 1-9 9"/>"#
        }
        ShellIconKind::Portfolio => {
            r#"<path d="M16 20V4a2 2 0 0 0-2-2h-4a2 2 0 0 0-2 2v16"/><rect width="20" height="14" x="2" y="6" rx="2"/>"#
        }
        ShellIconKind::Settings => {
            r#"<path d="M9.671 4.136a2.34 2.34 0 0 1 4.659 0 2.34 2.34 0 0 0 3.319 1.915 2.34 2.34 0 0 1 2.33 4.033 2.34 2.34 0 0 0 0 3.831 2.34 2.34 0 0 1-2.33 4.033 2.34 2.34 0 0 0-3.319 1.915 2.34 2.34 0 0 1-4.659 0 2.34 2.34 0 0 0-3.32-1.915 2.34 2.34 0 0 1-2.33-4.033 2.34 2.34 0 0 0 0-3.831A2.34 2.34 0 0 1 6.35 6.051a2.34 2.34 0 0 0 3.319-1.915"/><circle cx="12" cy="12" r="3"/>"#
        }
        ShellIconKind::Menu => r#"<path d="M4 6h16M4 12h16M4 18h16"/>"#,
        ShellIconKind::Bell => {
            r#"<path d="M10.268 21a2 2 0 0 0 3.464 0"/><path d="M3.262 15.326A1 1 0 0 0 4 17h16a1 1 0 0 0 .74-1.673C19.41 13.956 18 12.499 18 8A6 6 0 0 0 6 8c0 4.499-1.411 5.956-2.738 7.326"/>"#
        }
        ShellIconKind::Launcher => {
            r#"<rect width="3" height="3" x="4" y="4"/><rect width="3" height="3" x="10.5" y="4"/><rect width="3" height="3" x="17" y="4"/><rect width="3" height="3" x="4" y="10.5"/><rect width="3" height="3" x="10.5" y="10.5"/><rect width="3" height="3" x="17" y="10.5"/><rect width="3" height="3" x="4" y="17"/><rect width="3" height="3" x="10.5" y="17"/><rect width="3" height="3" x="17" y="17"/>"#
        }
        ShellIconKind::Star => {
            r#"<path d="m12 2 3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z"/>"#
        }
        ShellIconKind::More => {
            r#"<circle cx="5" cy="12" r="1" fill="currentColor" stroke="none"/><circle cx="12" cy="12" r="1" fill="currentColor" stroke="none"/><circle cx="19" cy="12" r="1" fill="currentColor" stroke="none"/>"#
        }
        ShellIconKind::Collapse => r#"<path d="m11 17-5-5 5-5M18 17l-5-5 5-5"/>"#,
    };

    view! {
        <svg
            class=class
            width="24"
            height="24"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1.5"
            stroke-linecap="round"
            stroke-linejoin="round"
            aria-hidden="true"
            focusable="false"
            inner_html=elements
        >
        </svg>
    }
}
