use hexagonal_backend::hexagon::{
    domain::volatility::TermStructureSource,
    driving_ports::for_analyzing_options::ForAnalyzingOptions,
};

#[tokio::test]
#[ignore = "requer o snapshot local do SPY em data/market_data.duckdb"]
async fn validates_real_cboe_volatility_snapshots() {
    let configured = hexagonal_backend::configurator::configure();
    let spy_term_structure = configured.options.term_structure("SPY").await.unwrap();
    let spy_30_days = &spy_term_structure.points[0];
    println!("SPY 30 dias: volatilidade={:.6}%", spy_30_days.volatility,);
    assert_eq!(spy_30_days.days, 30.0);
    assert!(matches!(
        spy_30_days.source,
        TermStructureSource::Interpolated { .. }
    ));

    let aapl_term_structure = configured.options.term_structure("AAPL").await.unwrap();
    let aapl_30_days = &aapl_term_structure.points[0];
    println!("AAPL 30 dias: volatilidade={:.6}%", aapl_30_days.volatility,);
    assert!(aapl_30_days.volatility.is_finite());
    assert!(aapl_30_days.volatility > 0.0);

    let ibm_term_structure = configured.options.term_structure("IBM").await.unwrap();
    let ibm_30_days = &ibm_term_structure.points[0];
    println!("IBM 30 dias: volatilidade={:.6}%", ibm_30_days.volatility,);
    assert!(ibm_30_days.volatility.is_finite());
    assert!(ibm_30_days.volatility > 0.0);
}
