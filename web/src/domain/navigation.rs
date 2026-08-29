#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NavItem {
    pub label: &'static str,
    pub href: &'static str,
    pub match_prefix: &'static str,
    pub separator_before: bool,
}

pub const GLOBAL_NAV: [NavItem; 9] = [
    NavItem::new("Dashboard", "/", "/", false),
    NavItem::new("Markets", "/markets", "/markets", false),
    NavItem::new("Assets", "/assets", "/assets", false),
    NavItem::new("Options", "/options", "/options", false),
    NavItem::new("Volatility", "/volatility", "/volatility", false),
    NavItem::new("GEX / Flow", "/gex", "/gex", false),
    NavItem::new("Simulations", "/simulations", "/simulations", false),
    NavItem::new("Portfolio", "/portfolio", "/portfolio", true),
    NavItem::new("Settings", "/settings", "/settings", false),
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
    const fn new(
        label: &'static str,
        href: &'static str,
        match_prefix: &'static str,
        separator_before: bool,
    ) -> Self {
        Self {
            label,
            href,
            match_prefix,
            separator_before,
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

    #[test]
    fn global_navigation_has_approved_order_destinations_and_separator() {
        assert_eq!(
            GLOBAL_NAV.map(|item| item.label),
            [
                "Dashboard",
                "Markets",
                "Assets",
                "Options",
                "Volatility",
                "GEX / Flow",
                "Simulations",
                "Portfolio",
                "Settings"
            ]
        );
        assert_eq!(
            GLOBAL_NAV.map(|item| item.href),
            [
                "/",
                "/markets",
                "/assets",
                "/options",
                "/volatility",
                "/gex",
                "/simulations",
                "/portfolio",
                "/settings"
            ]
        );
        assert!(GLOBAL_NAV[7].separator_before);
        assert_eq!(
            GLOBAL_NAV
                .iter()
                .filter(|item| item.separator_before)
                .count(),
            1
        );
        assert_eq!(GLOBAL_NAV.len() + 1, 10);
        for item in GLOBAL_NAV {
            assert!(item.is_current(item.href));
        }
    }
}
