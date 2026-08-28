#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FeedbackKind {
    Loading,
    Empty,
    Stale,
    Unavailable,
    RecoverableError,
    TerminalError,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FeedbackState {
    pub kind: FeedbackKind,
    pub title: &'static str,
    pub detail: &'static str,
}

impl FeedbackState {
    pub const fn loading() -> Self {
        Self::new(
            FeedbackKind::Loading,
            "Loading",
            "Preparing this workspace view.",
        )
    }
    pub const fn empty() -> Self {
        Self::new(
            FeedbackKind::Empty,
            "No content",
            "There is nothing to display yet.",
        )
    }
    pub const fn stale() -> Self {
        Self::new(
            FeedbackKind::Stale,
            "Data may be stale",
            "Refresh when a live contract is available.",
        )
    }
    pub const fn unavailable() -> Self {
        Self::new(
            FeedbackKind::Unavailable,
            "Unavailable",
            "This capability is not available for this view.",
        )
    }
    pub const fn recoverable_error() -> Self {
        Self::new(
            FeedbackKind::RecoverableError,
            "Could not load",
            "Try the action again.",
        )
    }
    pub const fn terminal_error() -> Self {
        Self::new(
            FeedbackKind::TerminalError,
            "View failed",
            "Return to workspace navigation.",
        )
    }
    const fn new(kind: FeedbackKind, title: &'static str, detail: &'static str) -> Self {
        Self {
            kind,
            title,
            detail,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn feedback_states_are_semantically_distinct() {
        let states = [
            FeedbackState::loading(),
            FeedbackState::empty(),
            FeedbackState::stale(),
            FeedbackState::unavailable(),
            FeedbackState::recoverable_error(),
            FeedbackState::terminal_error(),
        ];
        for (index, state) in states.iter().enumerate() {
            assert!(states[..index].iter().all(|other| other.kind != state.kind));
            assert!(!state.title.is_empty() && !state.detail.is_empty());
        }
    }
}
