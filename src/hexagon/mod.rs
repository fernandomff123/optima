//! The technology-neutral application and its complete boundary.

use std::{error::Error, fmt};

pub mod application;
pub mod domain;
pub mod driven_ports;
pub mod driving_ports;

/// Error expressed in application language at every port.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PortError {
    Conflict(String),
    InvalidRequest(String),
    NotFound(String),
    Unavailable(String),
}

impl fmt::Display for PortError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Conflict(message)
            | Self::InvalidRequest(message)
            | Self::NotFound(message)
            | Self::Unavailable(message) => formatter.write_str(message),
        }
    }
}

impl Error for PortError {}

pub type PortResult<T> = Result<T, PortError>;
