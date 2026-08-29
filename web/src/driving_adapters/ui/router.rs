use super::pages::{
    AssetChartPage, AssetGexPage, AssetOptionsPage, AssetOverviewPage, AssetRedirect,
    AssetSimulationPage, AssetVolatilityPage, AssetsPage, DashboardPage, GexPage, MarketsPage,
    NotFoundPage, OptionsPage, PortfolioPage, SettingsPage, SimulationsPage, VolatilityPage,
};
use leptos::prelude::*;
use leptos_router::{
    PossibleRouteMatch,
    components::{FlatRoutes, Route},
    path,
};

fn asset_root_route() -> impl PossibleRouteMatch + Clone {
    path!("/assets/:ticker")
}

fn asset_overview_route() -> impl PossibleRouteMatch + Clone {
    path!("/assets/:ticker/overview")
}

fn asset_chart_route() -> impl PossibleRouteMatch + Clone {
    path!("/assets/:ticker/chart")
}
fn options_route() -> impl PossibleRouteMatch + Clone {
    path!("/options")
}
fn volatility_route() -> impl PossibleRouteMatch + Clone {
    path!("/volatility")
}
fn gex_route() -> impl PossibleRouteMatch + Clone {
    path!("/gex")
}
fn simulations_route() -> impl PossibleRouteMatch + Clone {
    path!("/simulations")
}

#[component]
pub fn AppRoutes() -> impl IntoView {
    view! {
        <FlatRoutes fallback=NotFoundPage>
            <Route path=path!("") view=DashboardPage />
            <Route path=path!("markets") view=MarketsPage />
            <Route path=path!("assets") view=AssetsPage />
            <Route path=options_route() view=OptionsPage />
            <Route path=volatility_route() view=VolatilityPage />
            <Route path=gex_route() view=GexPage />
            <Route path=simulations_route() view=SimulationsPage />
            <Route path=asset_root_route() view=AssetRedirect />
            <Route path=asset_overview_route() view=AssetOverviewPage />
            <Route path=asset_chart_route() view=AssetChartPage />
            <Route path=path!("assets/:ticker/options") view=AssetOptionsPage />
            <Route path=path!("assets/:ticker/volatility") view=AssetVolatilityPage />
            <Route path=path!("assets/:ticker/gex") view=AssetGexPage />
            <Route path=path!("assets/:ticker/simulation") view=AssetSimulationPage />
            <Route path=path!("portfolio") view=PortfolioPage />
            <Route path=path!("settings") view=SettingsPage />
        </FlatRoutes>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use leptos_router::{NestedRoute, PathSegment, RouteDefs};

    fn runtime_asset_routes() -> RouteDefs<impl leptos_router::MatchNestedRoutes> {
        RouteDefs::new((
            NestedRoute::new(asset_root_route(), || ()),
            NestedRoute::new(asset_overview_route(), || ()),
            NestedRoute::new(asset_chart_route(), || ()),
        ))
    }

    fn runtime_global_routes() -> RouteDefs<impl leptos_router::MatchNestedRoutes> {
        RouteDefs::new((
            NestedRoute::new(options_route(), || ()),
            NestedRoute::new(volatility_route(), || ()),
            NestedRoute::new(gex_route(), || ()),
            NestedRoute::new(simulations_route(), || ()),
        ))
    }

    #[test]
    fn runtime_global_routes_support_direct_deep_links() {
        let routes = runtime_global_routes();
        let (_, generated) = routes.generate_routes();
        let generated = generated
            .into_iter()
            .map(|route| route.segments)
            .collect::<Vec<_>>();
        for path in ["/options", "/volatility", "/gex", "/simulations"] {
            assert!(recognizes(&generated, path));
        }
        assert!(!recognizes(&generated, "/options/not-a-route"));
    }

    #[test]
    fn runtime_asset_routes_match_paths_and_ignore_query() {
        let routes = runtime_asset_routes();
        let base = url::Url::parse(concat!("http", "://127.0.0.1:8080")).unwrap();
        let overview = base.join("/assets/SPX/overview").unwrap();
        let with_query = base.join("/assets/SPX/overview?scenario=normal").unwrap();
        let invalid = base.join("/assets/SPX/not-a-route").unwrap();
        let (_, generated) = routes.generate_routes();
        let generated = generated
            .into_iter()
            .map(|route| route.segments)
            .collect::<Vec<_>>();

        assert!(recognizes(&generated, overview.path()));
        assert!(recognizes(&generated, with_query.path()));
        assert_eq!(with_query.query(), Some("scenario=normal"));
        assert!(recognizes(&generated, "/assets/SPX"));
        assert!(recognizes(&generated, "/assets/SPX/chart"));
        for scenario in ["partial", "recoverable-error"] {
            let url = base
                .join(&format!("/assets/SPX/overview?scenario={scenario}"))
                .unwrap();
            assert!(recognizes(&generated, url.path()));
        }
        assert!(!recognizes(&generated, invalid.path()));
    }

    fn recognizes(routes: &[Vec<PathSegment>], path: &str) -> bool {
        let values = path.trim_matches('/').split('/').collect::<Vec<_>>();
        routes.iter().any(|segments| {
            segments.len() == values.len()
                && segments
                    .iter()
                    .zip(&values)
                    .all(|(segment, value)| match segment {
                        PathSegment::Static(expected) => expected == value,
                        PathSegment::Param(_) => !value.is_empty(),
                        _ => false,
                    })
        })
    }
}
