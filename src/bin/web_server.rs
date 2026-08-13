use std::{error::Error, sync::Arc, time::Duration};

use hexagonal_backend::hexagon::driving_ports::{
    for_refreshing_market_data::{DataRefreshTrigger, ForRefreshingMarketData},
    for_scheduling_market_operations::ForSchedulingMarketOperations,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let _ = dotenvy::from_filename(".env.local");

    // This is the server's only composition call. Every runtime actor receives
    // a port from this same configured object.
    let configured = hexagonal_backend::configurator::configure();
    hexagonal_backend::configurator::initialize_analytical_storage_with_config(
        &configured.composition_config,
    )
    .await?;
    let market_scheduling = configured.market_scheduling.clone();
    let market_open = market_scheduling
        .market_is_open(chrono::Utc::now())
        .unwrap_or(false);
    let (market_session_updates, market_session) = tokio::sync::watch::channel(market_open);
    let market_clock = tokio::spawn(run_market_session_clock(
        market_session_updates,
        market_scheduling,
    ));

    let data_refresh = configured.data_refresh.clone();
    run_startup_refresh(data_refresh.clone()).await?;
    let market_eod = tokio::spawn(run_market_eod_scheduler(data_refresh));

    let app = hexagonal_backend::configurator::configure_server_http_application(
        configured,
        market_session,
    );
    let address =
        std::env::var("HEXAGONAL_BACKEND_ADDR").unwrap_or_else(|_| "127.0.0.1:3100".to_string());
    let listener = tokio::net::TcpListener::bind(&address).await?;
    println!("API disponível em http://{address}");
    let result = axum::serve(listener, app).await;

    market_clock.abort();
    let _ = market_clock.await;
    market_eod.abort();
    let _ = market_eod.await;
    result?;
    Ok(())
}

async fn run_startup_refresh(
    application: Arc<dyn ForRefreshingMarketData>,
) -> hexagonal_backend::hexagon::PortResult<()> {
    let recovered = application
        .recover_interrupted_data_refreshes(chrono::Utc::now())
        .await?;
    if recovered > 0 {
        eprintln!("Foram reconciliadas {recovered} atualizações interrompidas");
    }
    if let Err(error) = application
        .request_data_refresh(DataRefreshTrigger::Startup, chrono::Utc::now())
        .await
    {
        eprintln!("Falha na atualização de arranque: {error}");
    }
    Ok(())
}

async fn run_market_session_clock(
    session: tokio::sync::watch::Sender<bool>,
    scheduling: hexagonal_backend::configurator::ConfiguredMarketScheduling,
) {
    loop {
        let now = chrono::Utc::now();
        if let Ok(open) = scheduling.market_is_open(now)
            && *session.borrow() != open
        {
            session.send_replace(open);
        }
        tokio::time::sleep_until(next_market_transition_deadline(&scheduling)).await;
    }
}

async fn run_market_eod_scheduler(application: Arc<dyn ForRefreshingMarketData>) {
    loop {
        let now = chrono::Utc::now();
        let wait = match hexagonal_backend::driving_adapters::scheduler::next_data_refresh_delay(
            application.as_ref(),
            now,
        )
        .await
        {
            Ok(wait) => wait,
            Err(error) => {
                eprintln!("Falha ao consultar próxima atualização: {error}");
                Duration::from_secs(3_600)
            }
        };
        tokio::time::sleep(wait).await;
        if let Err(error) = application
            .request_data_refresh(DataRefreshTrigger::Scheduler, chrono::Utc::now())
            .await
        {
            eprintln!("Falha na reconciliação EOD: {error}");
        }
    }
}

fn next_market_transition_deadline(
    scheduling: &impl ForSchedulingMarketOperations,
) -> tokio::time::Instant {
    let now = chrono::Utc::now();
    let wait = scheduling
        .next_market_transition(now)
        .ok()
        .and_then(|transition| (transition - now).to_std().ok())
        .unwrap_or_else(|| Duration::from_secs(3_600));
    tokio::time::Instant::now() + wait
}
