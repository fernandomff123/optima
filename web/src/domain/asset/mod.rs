#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssetSymbol(String);

impl AssetSymbol {
    pub fn new(value: impl Into<String>) -> Self {
        let value = value.into().trim().to_uppercase();
        Self(if value.is_empty() {
            "SPX".into()
        } else {
            value
        })
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AssetCapability {
    Overview,
    Chart,
    Options,
    Volatility,
    Gex,
    Simulation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AssetKind {
    Equity,
    Index,
}

impl AssetCapability {
    pub const fn segment(self) -> &'static str {
        match self {
            Self::Overview => "overview",
            Self::Chart => "chart",
            Self::Options => "options",
            Self::Volatility => "volatility",
            Self::Gex => "gex",
            Self::Simulation => "simulation",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Overview => "Overview",
            Self::Chart => "Chart",
            Self::Options => "Options",
            Self::Volatility => "Volatility",
            Self::Gex => "GEX",
            Self::Simulation => "Simulation",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symbol_normalizes_route_input() {
        assert_eq!(AssetSymbol::new(" spx ").as_str(), "SPX");
        assert_eq!(AssetSymbol::new("").as_str(), "SPX");
    }
}
