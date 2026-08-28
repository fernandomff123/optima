#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NavItem {
    pub label: &'static str,
    pub href: &'static str,
    pub match_prefix: &'static str,
}

pub const GLOBAL_NAV: [NavItem; 5] = [
    NavItem::new("Dashboard", "/", "/"),
    NavItem::new("Markets", "/markets", "/markets"),
    NavItem::new("Assets", "/assets", "/assets"),
    NavItem::new("Portfolio", "/portfolio", "/portfolio"),
    NavItem::new("Settings", "/settings", "/settings"),
];

pub const ASSET_TABS: [(&str, &str); 6] = [
    ("Overview", "overview"),
    ("Chart", "chart"),
    ("Options", "options"),
    ("Volatility", "volatility"),
    ("GEX", "gex"),
    ("Simulation", "simulation"),
];

impl NavItem {
    const fn new(label: &'static str, href: &'static str, match_prefix: &'static str) -> Self {
        Self {
            label,
            href,
            match_prefix,
        }
    }

    pub fn is_current(self, pathname: &str) -> bool {
        if self.href == "/" {
            pathname == "/"
        } else {
            pathname == self.match_prefix
                || pathname.starts_with(&format!("{}/", self.match_prefix))
        }
    }
}

pub fn asset_overview_path(ticker: &str) -> String {
    format!("/assets/{ticker}/overview")
}
pub fn asset_tab_path(ticker: &str, segment: &str) -> String {
    format!("/assets/{ticker}/{segment}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asset_root_resolves_to_overview() {
        assert_eq!(asset_overview_path("SPX"), "/assets/SPX/overview");
    }

    #[test]
    fn every_asset_tab_has_a_real_route() {
        for (_, segment) in ASSET_TABS {
            assert_eq!(
                asset_tab_path("SPX", segment),
                format!("/assets/SPX/{segment}")
            );
        }
    }

    #[test]
    fn global_selection_respects_route_boundaries() {
        let assets = GLOBAL_NAV[2];
        assert!(assets.is_current("/assets/SPX/chart"));
        assert!(!assets.is_current("/asset-prices"));
    }
}
