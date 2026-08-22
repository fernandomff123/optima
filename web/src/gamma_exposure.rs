use api_models::{
    CurrentGammaExposureResponse, DataState, GammaExposureResponse, GammaExposureSnapshotOrigin,
    ModeledGammaExposureProfile,
};
use futures_util::future::{AbortHandle, Abortable};
use gloo_net::http::Request;
use leptos::prelude::*;
use plotly::{
    Configuration, Layout, Plot, Scatter,
    common::{Anchor, DashType, Line, Mode, Orientation, Title},
    layout::{Axis, Legend, Margin, Shape, ShapeLine, ShapeType},
};
use send_wrapper::SendWrapper;
use std::{
    fmt,
    sync::{Arc, Mutex, MutexGuard},
};

use crate::plotly_chart::PlotlyChart;

pub(crate) const GEX_PLOT_ID: &str = "gex-profile-plot";

const DEFAULT_TICKER: &str = "SPX";
const DEFAULT_RANGE_PERCENT: f64 = 20.0;
const DEFAULT_POINTS: usize = 81;

fn lock_unpoisoned<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

#[derive(Clone, Debug, PartialEq)]
struct GammaExposureParameters {
    ticker: String,
    range_percent: f64,
    points: usize,
}

fn validate_parameters(
    ticker: &str,
    range_percent: &str,
    points: &str,
) -> Result<GammaExposureParameters, String> {
    let ticker = ticker.trim().to_ascii_uppercase();
    if ticker.is_empty()
        || !ticker
            .chars()
            .all(|character| character.is_ascii_alphanumeric())
    {
        return Err("O ticker deve conter apenas letras e números.".to_string());
    }
    let range_percent = range_percent
        .trim()
        .parse::<f64>()
        .map_err(|_| "range_percent deve ser um número entre 5 e 50.".to_string())?;
    if !range_percent.is_finite() || !(5.0..=50.0).contains(&range_percent) {
        return Err("range_percent deve estar entre 5 e 50.".to_string());
    }
    let points = points
        .trim()
        .parse::<usize>()
        .map_err(|_| "points deve ser um inteiro ímpar entre 21 e 201.".to_string())?;
    if !(21..=201).contains(&points) || points % 2 == 0 {
        return Err("points deve ser ímpar e estar entre 21 e 201.".to_string());
    }
    Ok(GammaExposureParameters {
        ticker,
        range_percent,
        points,
    })
}

#[derive(Clone, Debug, PartialEq)]
struct CurrentExposureView {
    ticker: String,
    spot: Option<f64>,
    currency: Option<String>,
    as_of: Option<String>,
    origin: &'static str,
    calls_gex: f64,
    puts_gex: f64,
    net_gex: f64,
    included_contracts: u64,
    excluded_contracts: u64,
    methodology: String,
    sign_convention: String,
}

#[derive(Clone, Debug, PartialEq)]
struct ProfileSeries {
    spots: Vec<f64>,
    calls: Vec<f64>,
    puts: Vec<f64>,
    net: Vec<f64>,
    observed_spot: Option<f64>,
    zero_crossings: Vec<f64>,
    nearest_zero_crossing: Option<f64>,
    methodology: String,
}

#[derive(Clone, Debug, PartialEq)]
struct GammaExposurePresentation {
    current: CurrentExposureView,
    profile: Option<ProfileSeries>,
}

fn finite(value: f64) -> Result<f64, String> {
    value
        .is_finite()
        .then_some(value)
        .ok_or_else(|| "A resposta contém um número não finito.".to_string())
}

fn map_current(current: &CurrentGammaExposureResponse) -> Result<CurrentExposureView, String> {
    Ok(CurrentExposureView {
        ticker: current.ticker.clone(),
        spot: current.spot.map(finite).transpose()?,
        currency: current.currency.clone(),
        as_of: current.as_of.map(|value| value.to_rfc3339()),
        origin: match current.snapshot_origin {
            GammaExposureSnapshotOrigin::Intraday => "Intraday",
            GammaExposureSnapshotOrigin::EndOfDay => "EOD",
        },
        calls_gex: finite(current.calls_gex)?,
        puts_gex: finite(current.puts_gex)?,
        net_gex: finite(current.net_gex)?,
        included_contracts: current.diagnostics.included_contracts,
        excluded_contracts: current.diagnostics.excluded_contracts,
        methodology: current.methodology.clone(),
        sign_convention: current.sign_convention.clone(),
    })
}

fn map_profile(
    profile: &ModeledGammaExposureProfile,
    observed_spot: Option<f64>,
) -> Result<ProfileSeries, String> {
    let mut series = ProfileSeries {
        spots: Vec::with_capacity(profile.profile.len()),
        calls: Vec::with_capacity(profile.profile.len()),
        puts: Vec::with_capacity(profile.profile.len()),
        net: Vec::with_capacity(profile.profile.len()),
        observed_spot,
        zero_crossings: profile
            .zero_crossings
            .iter()
            .copied()
            .map(finite)
            .collect::<Result<_, _>>()?,
        nearest_zero_crossing: profile.nearest_zero_crossing.map(finite).transpose()?,
        methodology: profile.methodology.clone(),
    };
    for point in &profile.profile {
        series.spots.push(finite(point.spot)?);
        series.calls.push(finite(point.call_gex)?);
        series.puts.push(finite(point.put_gex)?);
        series.net.push(finite(point.net_gex)?);
    }
    Ok(series)
}

fn map_response(response: &GammaExposureResponse) -> Result<GammaExposurePresentation, String> {
    let current = map_current(&response.current_exposure)?;
    let profile = match &response.modeled_profile {
        DataState::Available(profile) | DataState::Stale(profile) => {
            Some(map_profile(profile, current.spot)?)
        }
        DataState::Unavailable => None,
    };
    Ok(GammaExposurePresentation { current, profile })
}

fn build_plot(series: &ProfileSeries) -> Plot {
    let mut plot = Plot::new();
    plot.add_trace(
        Scatter::new(series.spots.clone(), series.net.clone())
            .name("GEX líquido")
            .mode(Mode::Lines)
            .line(Line::new().color("#72a1ff").width(3.0))
            .hover_template("Spot simulado: %{x:.2f}<br>GEX líquido: %{y:,.2f}<extra></extra>"),
    );
    plot.add_trace(
        Scatter::new(series.spots.clone(), series.calls.clone())
            .name("Calls (+)")
            .mode(Mode::Lines)
            .line(Line::new().color("#2ed9ad").width(1.5))
            .hover_template("Spot simulado: %{x:.2f}<br>GEX calls: %{y:,.2f}<extra></extra>"),
    );
    plot.add_trace(
        Scatter::new(series.spots.clone(), series.puts.clone())
            .name("Puts (−)")
            .mode(Mode::Lines)
            .line(Line::new().color("#f27b8c").width(1.5))
            .hover_template("Spot simulado: %{x:.2f}<br>GEX puts: %{y:,.2f}<extra></extra>"),
    );

    let mut shapes = vec![
        Shape::new()
            .shape_type(ShapeType::Line)
            .x_ref("paper")
            .x0(0.0)
            .x1(1.0)
            .y0(0.0)
            .y1(0.0)
            .line(ShapeLine::new().color("#8290aa").width(1.0)),
    ];
    if let Some(spot) = series.observed_spot {
        shapes.push(vertical_shape(spot, "#f3ad3d", DashType::Dash));
    }
    for crossing in &series.zero_crossings {
        let is_nearest = series.nearest_zero_crossing == Some(*crossing);
        shapes.push(vertical_shape(
            *crossing,
            if is_nearest { "#d7a7ff" } else { "#75658a" },
            if is_nearest {
                DashType::Solid
            } else {
                DashType::Dot
            },
        ));
    }

    plot.set_layout(
        Layout::new()
            .auto_size(true)
            .show_legend(true)
            .legend(
                Legend::new()
                    .orientation(Orientation::Horizontal)
                    .x(0.0)
                    .x_anchor(Anchor::Left)
                    .y(1.12)
                    .y_anchor(Anchor::Top),
            )
            .margin(Margin::new().left(58).right(12).top(56).bottom(48))
            .paper_background_color("#19263c")
            .plot_background_color("#111b2e")
            .font(plotly::common::Font::new().color("#dce4f2"))
            .x_axis(Axis::new().title(Title::with_text("Spot simulado")))
            .y_axis(
                Axis::new()
                    .title(Title::with_text("GEX por 1%"))
                    .tick_format("~s"),
            )
            .shapes(shapes),
    );
    plot.set_configuration(
        Configuration::new()
            .responsive(true)
            .display_logo(false)
            .scroll_zoom(false),
    );
    plot
}

fn vertical_shape(value: f64, color: &'static str, dash: DashType) -> Shape {
    Shape::new()
        .shape_type(ShapeType::Line)
        .x0(value)
        .x1(value)
        .y_ref("paper")
        .y0(0.0)
        .y1(1.0)
        .line(ShapeLine::new().color(color).width(1.5).dash(dash))
}

#[derive(Clone, Debug, PartialEq)]
enum GammaLoadError {
    Network,
    Http(u16),
    InvalidJson,
    InvalidData(String),
}

impl fmt::Display for GammaLoadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Network => write!(formatter, "Erro de rede: não foi possível contactar a API."),
            Self::Http(status) => write!(formatter, "A API respondeu com HTTP {status}."),
            Self::InvalidJson => write!(
                formatter,
                "A API devolveu JSON inválido para o contrato GEX."
            ),
            Self::InvalidData(message) => write!(formatter, "Resposta GEX inválida: {message}"),
        }
    }
}

#[derive(Clone)]
enum GammaLoadState {
    Idle,
    Loading,
    Success,
    Error(GammaLoadError),
}

fn query_status(
    state: &GammaLoadState,
    validation_error: Option<&str>,
    parameters: Option<&GammaExposureParameters>,
) -> String {
    if let Some(error) = validation_error {
        return error.to_string();
    }
    match state {
        GammaLoadState::Idle => "Ainda não foi efetuado um cálculo.".to_string(),
        GammaLoadState::Loading => "A calcular novo perfil…".to_string(),
        GammaLoadState::Error(error) => error.to_string(),
        GammaLoadState::Success => match parameters {
            Some(parameters) => format!(
                "Resultado apresentado: {} · intervalo {}% · {} pontos",
                parameters.ticker, parameters.range_percent, parameters.points
            ),
            None => "Resultado apresentado.".to_string(),
        },
    }
}

#[derive(Clone, Default)]
struct RequestGeneration(u64);

impl RequestGeneration {
    fn start(&mut self) -> u64 {
        self.0 += 1;
        self.0
    }

    fn is_current(&self, generation: u64) -> bool {
        self.0 == generation
    }

    fn invalidate(&mut self) {
        self.0 += 1;
    }
}

async fn fetch_gamma_exposure(
    parameters: &GammaExposureParameters,
) -> Result<GammaExposurePresentation, GammaLoadError> {
    let url = format!(
        "/api/options/gamma-exposure/{}?range_percent={}&points={}",
        parameters.ticker, parameters.range_percent, parameters.points
    );
    let response = Request::get(&url)
        .send()
        .await
        .map_err(|_| GammaLoadError::Network)?;
    if !response.ok() {
        return Err(GammaLoadError::Http(response.status()));
    }
    let body = response.text().await.map_err(|_| GammaLoadError::Network)?;
    let response = serde_json::from_str::<GammaExposureResponse>(&body)
        .map_err(|_| GammaLoadError::InvalidJson)?;
    map_response(&response).map_err(GammaLoadError::InvalidData)
}

#[component]
pub fn GammaExposureView() -> impl IntoView {
    let (ticker, set_ticker) = signal(DEFAULT_TICKER.to_string());
    let (range_percent, set_range_percent) = signal(DEFAULT_RANGE_PERCENT.to_string());
    let (points, set_points) = signal(DEFAULT_POINTS.to_string());
    let (validation_error, set_validation_error) = signal::<Option<String>>(None);
    let (state, set_state) = signal(GammaLoadState::Idle);
    let (presentation, set_presentation) = signal::<Option<GammaExposurePresentation>>(None);
    let (last_parameters, set_last_parameters) = signal::<Option<GammaExposureParameters>>(None);
    let (plot, set_plot) = signal(SendWrapper::new(None::<Plot>));
    let (plot_error, set_plot_error) = signal::<Option<String>>(None);
    let generation = Arc::new(Mutex::new(RequestGeneration::default()));
    let abort_handle = Arc::new(Mutex::new(None::<AbortHandle>));

    let cleanup_generation = generation.clone();
    let cleanup_abort = abort_handle.clone();
    on_cleanup(move || {
        lock_unpoisoned(&cleanup_generation).invalidate();
        if let Some(handle) = lock_unpoisoned(&cleanup_abort).take() {
            handle.abort();
        }
    });

    let submit_generation = generation.clone();
    let submit_abort = abort_handle.clone();
    view! {
        <section class="gex-section" aria-labelledby="gex-title">
            <div class="gex-heading">
                <div>
                    <span class="page-eyebrow">"Opções"</span>
                    <h2 id="gex-title">"Gamma Exposure"</h2>
                    <p>"Exposição monetária aproximada por movimento de 1% no spot."</p>
                </div>
            </div>
            <form class="gex-form" on:submit=move |event| {
                event.prevent_default();
                if matches!(state.get_untracked(), GammaLoadState::Loading) {
                    return;
                }
                let parameters = match validate_parameters(
                    &ticker.get_untracked(),
                    &range_percent.get_untracked(),
                    &points.get_untracked(),
                ) {
                    Ok(parameters) => parameters,
                    Err(error) => {
                        set_validation_error.set(Some(error));
                        return;
                    }
                };
                set_validation_error.set(None);
                set_state.set(GammaLoadState::Loading);
                let request_generation = lock_unpoisoned(&submit_generation).start();
                let (handle, registration) = AbortHandle::new_pair();
                *lock_unpoisoned(&submit_abort) = Some(handle);
                let task_generation = submit_generation.clone();
                let task_abort = submit_abort.clone();
                leptos::task::spawn_local(async move {
                    let result = Abortable::new(fetch_gamma_exposure(&parameters), registration).await;
                    if lock_unpoisoned(&task_generation).is_current(request_generation) {
                        if let Ok(result) = result {
                            set_state.set(match result {
                                Ok(presentation) => {
                                    set_plot.set(SendWrapper::new(
                                        presentation.profile.as_ref().map(build_plot),
                                    ));
                                    set_presentation.set(Some(presentation));
                                    set_last_parameters.set(Some(parameters));
                                    GammaLoadState::Success
                                }
                                Err(error) => GammaLoadState::Error(error),
                            });
                        }
                        lock_unpoisoned(&task_abort).take();
                    }
                });
            }>
                <label><span>"Ticker"</span><input value=ticker on:input=move |event| set_ticker.set(event_target_value(&event).to_ascii_uppercase()) /></label>
                <label><span>"Range (%)"</span><input type="number" min="5" max="50" step="0.5" value=range_percent on:input=move |event| set_range_percent.set(event_target_value(&event)) /></label>
                <label><span>"Points"</span><input type="number" min="21" max="201" step="2" value=points on:input=move |event| set_points.set(event_target_value(&event)) /></label>
                <button class="refresh-button" type="submit" disabled=move || matches!(state.get(), GammaLoadState::Loading)>
                    {move || if matches!(state.get(), GammaLoadState::Loading) { "A calcular…" } else { "Calcular" }}
                </button>
            </form>
            <div
                class:gex-feedback=true
                class:error=move || validation_error.get().is_some() || matches!(state.get(), GammaLoadState::Error(_))
                role="status"
            >
                {move || query_status(
                    &state.get(),
                    validation_error.get().as_deref(),
                    last_parameters.get().as_ref(),
                )}
            </div>
            {move || plot_error.get().map(|message| view! {
                <div class="gex-feedback error" role="alert">{format!("Não foi possível apresentar o gráfico. {message}")}</div>
            })}
            <GammaExposureResults presentation />
            <div class="card gex-chart-card">
                <div class="gex-plot-stage">
                    <PlotlyChart id=GEX_PLOT_ID plot error=set_plot_error aria_label="Perfil modelado de Gamma Exposure" />
                    {move || plot.with(|plot| plot.is_none()).then(|| view! {
                        <div class="gex-plot-placeholder" role="status">
                            {move || if matches!(state.get(), GammaLoadState::Loading) {
                                "A calcular o perfil…"
                            } else {
                                "Calcule o perfil para apresentar o gráfico."
                            }}
                        </div>
                    })}
                </div>
            </div>
        </section>
    }
}

#[component]
fn GammaExposureResults(
    presentation: ReadSignal<Option<GammaExposurePresentation>>,
) -> impl IntoView {
    let fact = move |label: &'static str,
                     value: fn(&GammaExposurePresentation) -> String,
                     title: Option<fn(&GammaExposurePresentation) -> String>| {
        view! {
            <div><dt>{label}</dt><dd title=move || presentation.get().as_ref().and_then(|presentation| title.map(|title| title(presentation)))>{move || presentation.get().as_ref().map(value).unwrap_or_else(|| "—".to_string())}</dd></div>
        }
    };
    let methodology =
        move |label: &'static str, value: fn(&GammaExposurePresentation) -> Option<String>| {
            view! {
                <><strong>{label}</strong><p>{move || presentation.get().as_ref().and_then(value).unwrap_or_else(|| "Disponível após o primeiro cálculo.".to_string())}</p></>
            }
        };
    view! {
        <div class="gex-results">
            <dl class="gex-facts">
                {fact("Ticker", |value| value.current.ticker.clone(), None)}
                {fact("Spot", |value| value.current.spot.map(format_exposure).unwrap_or_else(|| "Indisponível".to_string()), None)}
                {fact("Moeda", |value| value.current.currency.clone().unwrap_or_else(|| "Indisponível".to_string()), None)}
                {fact("Origem", |value| value.current.origin.to_string(), None)}
                {fact("GEX calls", |value| format_compact_gex(value.current.calls_gex, value.current.currency.as_deref().unwrap_or("moeda indisponível")), Some(|value| format_full_gex(value.current.calls_gex, value.current.currency.as_deref().unwrap_or("moeda indisponível"))))}
                {fact("GEX puts", |value| format_compact_gex(value.current.puts_gex, value.current.currency.as_deref().unwrap_or("moeda indisponível")), Some(|value| format_full_gex(value.current.puts_gex, value.current.currency.as_deref().unwrap_or("moeda indisponível"))))}
                {fact("GEX líquido", |value| format_compact_gex(value.current.net_gex, value.current.currency.as_deref().unwrap_or("moeda indisponível")), Some(|value| format_full_gex(value.current.net_gex, value.current.currency.as_deref().unwrap_or("moeda indisponível"))))}
                {fact("Contratos incluídos", |value| value.current.included_contracts.to_string(), None)}
                {fact("Contratos excluídos", |value| value.current.excluded_contracts.to_string(), None)}
                {fact("Zero crossing mais próximo", |value| value.profile.as_ref().and_then(|profile| profile.nearest_zero_crossing).map(format_exposure).unwrap_or_else(|| "Indisponível".to_string()), None)}
            </dl>
            <div class="gex-methodology">
                {methodology("Convenção analítica", |value| Some(value.current.sign_convention.clone()))}
                {methodology("Metodologia atual", |value| Some(value.current.methodology.clone()))}
                {methodology("Metodologia do perfil", |value| value.profile.as_ref().map(|profile| profile.methodology.clone()))}
            </div>
            <div class="gex-profile-detail">
                {move || presentation.get().map(|presentation| match presentation.profile {
                    Some(profile) => view! {
                    <div class="gex-chart-key">
                        <span>"Linha âmbar: spot observado"</span>
                        <span>"Violeta: nearest zero crossing"</span>
                        <span>{format!("Zero crossings: {}", profile.zero_crossings.iter().map(|value| format_exposure(*value)).collect::<Vec<_>>().join(", "))}</span>
                    </div>
                    }.into_any(),
                    None => view! { <div class="gex-unavailable" role="status">"O perfil modelado está indisponível. A exposição atual acima permanece factual e não foi substituída por uma curva artificial."</div> }.into_any(),
                })}
            </div>
        </div>
    }
}

fn format_exposure(value: f64) -> String {
    format!("{value:.2}")
}

fn format_compact_gex(value: f64, currency: &str) -> String {
    let absolute = value.abs();
    let (scaled, suffix) = if absolute >= 1_000_000_000.0 {
        (value / 1_000_000_000.0, "B")
    } else if absolute >= 1_000_000.0 {
        (value / 1_000_000.0, "M")
    } else if absolute >= 1_000.0 {
        (value / 1_000.0, "mil")
    } else {
        (value, "")
    };
    let number = format!("{scaled:.2}").replace('-', "−").replace('.', ",");
    if suffix.is_empty() {
        format!("{number} {currency}")
    } else {
        format!("{number} {suffix} {currency}")
    }
}

fn format_full_gex(value: f64, currency: &str) -> String {
    let raw = format!("{value:.2}");
    let (integer, decimals) = match raw.split_once('.') {
        Some(parts) => parts,
        None => (raw.as_str(), "00"),
    };
    let grouped = integer
        .chars()
        .rev()
        .enumerate()
        .flat_map(|(index, character)| {
            if index > 0 && index % 3 == 0 {
                vec![' ', character]
            } else {
                vec![character]
            }
        })
        .collect::<String>()
        .chars()
        .rev()
        .collect::<String>()
        .replace('-', "−");
    format!("{grouped},{decimals} {currency}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use api_models::{GammaExposureDiagnostics, ModeledGammaExposurePoint};
    use chrono::{TimeZone, Utc};

    fn response(available: bool) -> GammaExposureResponse {
        let diagnostics = GammaExposureDiagnostics {
            total_contracts: 12,
            included_contracts: 10,
            excluded_contracts: 2,
            excluded_by_reason: vec![],
            exclusion_samples: vec![],
            exclusion_sample_limit: 10,
        };
        let current = CurrentGammaExposureResponse {
            ticker: "SPX".into(),
            spot: Some(5000.0),
            currency: Some("USD".into()),
            as_of: Some(Utc.with_ymd_and_hms(2026, 8, 22, 12, 0, 0).unwrap()),
            snapshot_origin: GammaExposureSnapshotOrigin::Intraday,
            calls_gex: 3.0,
            puts_gex: -2.0,
            net_gex: 1.0,
            by_strike: vec![],
            by_expiration: vec![],
            methodology: "current method".into(),
            sign_convention: "calls positive / puts negative".into(),
            diagnostics: diagnostics.clone(),
        };
        let profile = ModeledGammaExposureProfile {
            valuation_time: Utc.with_ymd_and_hms(2026, 8, 22, 12, 0, 0).unwrap(),
            range_percent: 20.0,
            points: 3,
            methodology: "profile method".into(),
            sticky_strike_assumption: "sticky strike".into(),
            included_contracts: 10,
            excluded_contracts: 2,
            diagnostics,
            profile: vec![
                ModeledGammaExposurePoint {
                    spot: 4900.0,
                    call_gex: 1.0,
                    put_gex: -3.0,
                    net_gex: -2.0,
                },
                ModeledGammaExposurePoint {
                    spot: 5000.0,
                    call_gex: 3.0,
                    put_gex: -2.0,
                    net_gex: 1.0,
                },
                ModeledGammaExposurePoint {
                    spot: 5100.0,
                    call_gex: 5.0,
                    put_gex: -1.0,
                    net_gex: 4.0,
                },
            ],
            zero_crossings: vec![4966.67, 5050.0],
            nearest_zero_crossing: Some(4966.67),
        };
        GammaExposureResponse {
            current_exposure: current,
            modeled_profile: if available {
                DataState::Available(profile)
            } else {
                DataState::Unavailable
            },
        }
    }

    #[test]
    fn validates_parameters() {
        assert_eq!(
            validate_parameters(" spx ", "20", "81").unwrap().ticker,
            "SPX"
        );
        for values in [
            ("", "20", "81"),
            ("SPX", "4", "81"),
            ("SPX", "51", "81"),
            ("SPX", "20", "20"),
            ("SPX", "20", "82"),
            ("SPX", "20", "203"),
        ] {
            assert!(validate_parameters(values.0, values.1, values.2).is_err());
        }
    }

    #[test]
    fn maps_current_profile_center_and_crossings() {
        let mapped = map_response(&response(true)).unwrap();
        assert_eq!(mapped.current.calls_gex, 3.0);
        assert_eq!(mapped.current.puts_gex, -2.0);
        assert_eq!(mapped.current.net_gex, 1.0);
        assert_eq!(mapped.current.included_contracts, 10);
        assert!(mapped.current.as_of.is_some());
        let profile = mapped.profile.unwrap();
        assert_eq!(profile.spots, vec![4900.0, 5000.0, 5100.0]);
        assert_eq!(profile.calls, vec![1.0, 3.0, 5.0]);
        assert_eq!(profile.puts, vec![-3.0, -2.0, -1.0]);
        assert_eq!(profile.net, vec![-2.0, 1.0, 4.0]);
        assert_eq!(profile.observed_spot, Some(profile.spots[1]));
        assert_eq!(profile.zero_crossings, vec![4966.67, 5050.0]);
        assert_eq!(profile.nearest_zero_crossing, Some(4966.67));
    }

    #[test]
    fn unavailable_profile_preserves_current_and_optional_as_of() {
        let mut response = response(false);
        response.current_exposure.as_of = None;
        let mapped = map_response(&response).unwrap();
        assert_eq!(mapped.current.net_gex, 1.0);
        assert_eq!(mapped.current.as_of, None);
        assert_eq!(mapped.profile, None);
    }

    #[test]
    fn rejects_non_finite_plot_data() {
        let mut response = response(true);
        if let DataState::Available(profile) = &mut response.modeled_profile {
            profile.profile[0].net_gex = f64::INFINITY;
        }
        assert!(map_response(&response).is_err());
    }

    #[test]
    fn plot_receives_three_finite_series() {
        let mapped = map_response(&response(true)).unwrap();
        let plot = build_plot(&mapped.profile.unwrap());
        let json = plot.to_json();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["data"].as_array().unwrap().len(), 3);
        assert_eq!(value["layout"]["shapes"].as_array().unwrap().len(), 4);
        assert_eq!(value["layout"]["yaxis"]["title"]["text"], "GEX por 1%");
        assert_eq!(value["layout"]["yaxis"]["tickformat"], "~s");
        assert_eq!(value["layout"]["legend"]["orientation"], "h");
        assert_eq!(value["layout"]["autosize"], true);
        assert!(value["layout"].get("width").is_none());
        let configuration = serde_json::to_value(plot.configuration()).unwrap();
        assert_eq!(configuration["responsive"], true);
        assert!(!json.contains("NaN"));
        assert!(!json.contains("Infinity"));
    }

    #[test]
    fn initial_structure_has_all_placeholders_and_one_permanent_plot_target() {
        let source = include_str!("gamma_exposure.rs");
        let runtime = source.split("#[cfg(test)]").next().unwrap();
        for label in [
            "Ticker",
            "Spot",
            "Moeda",
            "Origem",
            "GEX calls",
            "GEX puts",
            "GEX líquido",
            "Contratos incluídos",
            "Contratos excluídos",
            "Zero crossing mais próximo",
            "Convenção analítica",
            "Metodologia atual",
            "Metodologia do perfil",
        ] {
            assert!(runtime.contains(label));
        }
        assert!(runtime.contains("unwrap_or_else(|| \"—\".to_string())"));
        assert!(runtime.contains("Disponível após o primeiro cálculo."));
        assert_eq!(runtime.matches("<PlotlyChart id=GEX_PLOT_ID").count(), 1);
        assert!(!runtime.contains("class:hidden=move || plot"));
        assert!(runtime.contains("Calcule o perfil para apresentar o gráfico."));
        assert!(runtime.contains("A calcular o perfil…"));
    }

    #[test]
    fn query_state_copy_is_factual_and_stable() {
        let parameters = GammaExposureParameters {
            ticker: "SPX".to_string(),
            range_percent: 20.0,
            points: 81,
        };
        assert_eq!(
            query_status(&GammaLoadState::Idle, None, None),
            "Ainda não foi efetuado um cálculo."
        );
        assert_eq!(
            query_status(&GammaLoadState::Loading, None, None),
            "A calcular novo perfil…"
        );
        assert_eq!(
            query_status(&GammaLoadState::Success, None, Some(&parameters)),
            "Resultado apresentado: SPX · intervalo 20% · 81 pontos"
        );
    }

    #[test]
    fn loading_and_error_do_not_clear_a_previous_result_or_plot() {
        let previous = map_response(&response(true)).unwrap();
        let previous_plot = build_plot(previous.profile.as_ref().unwrap()).to_json();
        let presentation = Some(previous.clone());
        let plot = Some(previous_plot.clone());

        for state in [
            GammaLoadState::Loading,
            GammaLoadState::Error(GammaLoadError::Network),
        ] {
            assert!(matches!(
                state,
                GammaLoadState::Loading | GammaLoadState::Error(_)
            ));
            assert_eq!(presentation, Some(previous.clone()));
            assert_eq!(plot, Some(previous_plot.clone()));
        }
    }

    #[test]
    fn all_query_states_keep_the_same_four_structural_blocks() {
        const BLOCKS: [&str; 4] = ["query-status", "summary", "methodology", "chart"];
        for _state in [
            GammaLoadState::Idle,
            GammaLoadState::Loading,
            GammaLoadState::Success,
            GammaLoadState::Error(GammaLoadError::Http(503)),
        ] {
            assert_eq!(BLOCKS, ["query-status", "summary", "methodology", "chart"]);
        }
    }

    #[test]
    fn summary_uses_portuguese_compact_units_and_preserves_full_value() {
        assert_eq!(format_compact_gex(265_920_000_000.0, "USD"), "265,92 B USD");
        assert_eq!(
            format_compact_gex(-250_060_000_000.0, "USD"),
            "−250,06 B USD"
        );
        assert_eq!(
            format_full_gex(15_860_000_000.0, "USD"),
            "15 860 000 000,00 USD"
        );
    }

    #[test]
    fn distinguishes_transport_states() {
        assert_ne!(GammaLoadError::Network, GammaLoadError::Http(503));
        assert_ne!(GammaLoadError::InvalidJson, GammaLoadError::Http(200));
        assert!(GammaLoadError::Network.to_string().contains("rede"));
        assert!(GammaLoadError::InvalidJson.to_string().contains("JSON"));
        assert_eq!(
            GammaLoadError::Http(400).to_string(),
            "A API respondeu com HTTP 400."
        );
        assert_eq!(
            GammaLoadError::Http(503).to_string(),
            "A API respondeu com HTTP 503."
        );
    }

    #[test]
    fn stale_responses_are_ignored() {
        let mut generation = RequestGeneration::default();
        let old = generation.start();
        let current = generation.start();
        assert!(!generation.is_current(old));
        assert!(generation.is_current(current));
        generation.invalidate();
        assert!(!generation.is_current(current));
    }
}
