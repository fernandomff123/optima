//! Autonomous market and option backend organized as one hexagonal application.
//!
//! Start with the [`architecture_guide`] to explore actors, ports, adapters,
//! and the conversations coordinated by the application.

pub mod api;
pub mod architecture_guide;
pub mod configurator;
pub mod driven_adapters;
pub mod driving_adapters;
pub mod hexagon;
