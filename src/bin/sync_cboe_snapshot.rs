use hexagonal_backend::hexagon::driving_ports::for_scheduling_market_operations::ForSchedulingMarketOperations;
use hexagonal_backend::hexagon::driving_ports::for_synchronizing_market_data::ForSynchronizingMarketData;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    let calculate_term_structure = arguments
        .iter()
        .any(|argument| argument == "--term-structure");
    let ticker = arguments
        .iter()
        .find(|argument| argument.as_str() != "--term-structure")
        .cloned()
        .unwrap_or_else(|| "SPY".to_string());

    let configured = hexagonal_backend::configurator::configure();
    let Some(market_close) = configured
        .market_scheduling
        .eligible_end_of_day_close(chrono::Utc::now())?
    else {
        println!("{ticker}: sessão atual ainda não elegível");
        return Ok(());
    };
    let chain = configured
        .synchronization
        .synchronize_option_chain(&ticker, market_close)
        .await?;
    println!(
        "{ticker}: contratos obtidos={}, snapshots inseridos={}",
        chain.items_obtained, chain.items_stored
    );
    if calculate_term_structure {
        let term = configured
            .synchronization
            .synchronize_term_structure(&ticker)
            .await?;
        println!(
            "{ticker}: pontos calculados={}, pontos inseridos={}",
            term.items_obtained, term.items_stored
        );
    }

    Ok(())
}
