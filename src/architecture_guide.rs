//! Architecture documentation organized around actors and conversations.
//!
//! The source documents are also available under `docs/` for convenient
//! reading directly from the repository.

/// Driving and driven actors, their intentions, ports, and production adapters.
#[doc = include_str!("../docs/actors.md")]
pub mod actors {}

/// Use cases offered by the application and the coordination each one owns.
#[doc = include_str!("../docs/conversations.md")]
pub mod conversations {
    /// Choosing and combining observations to value portfolio positions.
    #[doc = include_str!("../docs/conversations/portfolio-valuation.md")]
    pub mod portfolio_valuation {}

    /// Obtaining, deriving, and persisting market observations.
    #[doc = include_str!("../docs/conversations/synchronization.md")]
    pub mod synchronization {}

    /// Loading stored option observations and applying domain analytics.
    #[doc = include_str!("../docs/conversations/options.md")]
    pub mod options {}

    /// Preparing intraday option data and streaming live prices.
    #[doc = include_str!("../docs/conversations/intraday-market.md")]
    pub mod intraday_market {}
}
