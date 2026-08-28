//! Shell icons copied from the official Lucide icon set.
//! Source details are recorded in the frontend README.
//! License: ISC (Lucide); Search is derived from Feather under MIT.

use leptos::prelude::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShellIconKind {
    Dashboard,
    Markets,
    Search,
    Portfolio,
    Settings,
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
        ShellIconKind::Portfolio => {
            r#"<path d="M16 20V4a2 2 0 0 0-2-2h-4a2 2 0 0 0-2 2v16"/><rect width="20" height="14" x="2" y="6" rx="2"/>"#
        }
        ShellIconKind::Settings => {
            r#"<path d="M9.671 4.136a2.34 2.34 0 0 1 4.659 0 2.34 2.34 0 0 0 3.319 1.915 2.34 2.34 0 0 1 2.33 4.033 2.34 2.34 0 0 0 0 3.831 2.34 2.34 0 0 1-2.33 4.033 2.34 2.34 0 0 0-3.319 1.915 2.34 2.34 0 0 1-4.659 0 2.34 2.34 0 0 0-3.32-1.915 2.34 2.34 0 0 1-2.33-4.033 2.34 2.34 0 0 0 0-3.831A2.34 2.34 0 0 1 6.35 6.051a2.34 2.34 0 0 0 3.319-1.915"/><circle cx="12" cy="12" r="3"/>"#
        }
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
