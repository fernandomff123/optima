//! Compatibility mapping for the pre-hexagonal portfolio HTTP contract.

use std::error::Error;

use api_models::{
    PortfolioCashOverview, PortfolioCashResponse, PortfolioMovementOverview,
    PortfolioMovementsResponse, PortfolioOverview, PortfolioPositionOverview,
    PortfolioPositionsResponse, PortfolioSummaryResponse,
};

use crate::hexagon::domain::portfolio::{
    CashMovementKind, Instrument, Portfolio, PortfolioEvent, TradeSide,
};

type ViewResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

pub fn overview(portfolio: Portfolio) -> ViewResult<PortfolioOverview> {
    let summary = summary(&portfolio)?;
    let cash = cash(&portfolio);
    let positions = positions_without_market(&portfolio);
    let movements = movements(&portfolio)?;
    Ok(PortfolioOverview {
        id: summary.id,
        name: summary.name,
        base_currency: summary.base_currency,
        event_count: summary.event_count,
        realized_gain_eur: summary.realized_gain_eur,
        net_value_eur: summary.net_value_eur,
        valuation_note: summary.valuation_note,
        realized_gains: summary.realized_gains,
        positions: positions.positions,
        cash_balances: cash.cash_balances,
        movements: movements.movements,
    })
}

pub fn summary(portfolio: &Portfolio) -> ViewResult<PortfolioSummaryResponse> {
    let realized_gain = portfolio
        .fifo_tax_projection()?
        .realized
        .iter()
        .map(|lot| lot.gain_eur)
        .sum::<rust_decimal::Decimal>();
    let net_value_eur = if portfolio.positions().is_empty() {
        Some(portfolio.cash_value_in_base()?.amount.to_string())
    } else {
        None
    };
    let valuation_note = net_value_eur
        .as_ref()
        .map(|_| "Caixa convertido pela taxa mais recente registada".to_string());
    Ok(PortfolioSummaryResponse {
        id: portfolio.id.clone(),
        name: portfolio.name.clone(),
        base_currency: portfolio.base_currency.code().to_string(),
        event_count: portfolio.events().len(),
        realized_gain_eur: realized_gain.to_string(),
        net_value_eur,
        valuation_note,
        realized_gains: portfolio
            .realized_gains_by_currency()?
            .into_iter()
            .map(|(currency, amount)| PortfolioCashOverview {
                currency,
                amount: amount.to_string(),
            })
            .collect(),
    })
}

pub fn cash(portfolio: &Portfolio) -> PortfolioCashResponse {
    PortfolioCashResponse {
        cash_balances: portfolio
            .cash_balances()
            .into_iter()
            .map(|(currency, amount)| PortfolioCashOverview {
                currency,
                amount: amount.to_string(),
            })
            .collect(),
    }
}

pub fn movements(portfolio: &Portfolio) -> ViewResult<PortfolioMovementsResponse> {
    Ok(PortfolioMovementsResponse {
        movements: portfolio
            .events()
            .iter()
            .map(movement)
            .collect::<ViewResult<Vec<_>>>()?,
    })
}

fn positions_without_market(portfolio: &Portfolio) -> PortfolioPositionsResponse {
    PortfolioPositionsResponse {
        positions: portfolio
            .positions()
            .into_iter()
            .map(|position| PortfolioPositionOverview {
                instrument: instrument_label(&position.instrument),
                quantity: position.quantity.to_string(),
                market_price: None,
                market_value: None,
                market_currency: None,
                market_source: None,
                market_time: None,
            })
            .collect(),
    }
}

fn movement(event: &PortfolioEvent) -> ViewResult<PortfolioMovementOverview> {
    let (
        kind,
        description,
        amount,
        currency,
        counter_amount,
        counter_currency,
        tax_amount_eur,
        tax_rate,
        tax_rate_date,
        tax_rate_source,
    ) = match event {
        PortfolioEvent::CashMovement(movement) => {
            let sign = if movement.kind == CashMovementKind::Deposit {
                rust_decimal::Decimal::ONE
            } else {
                -rust_decimal::Decimal::ONE
            };
            (
                "Caixa",
                match movement.kind {
                    CashMovementKind::Deposit => "Depósito".to_string(),
                    CashMovementKind::Withdrawal => "Levantamento".to_string(),
                },
                (sign * movement.amount.amount).to_string(),
                movement.amount.currency.code().to_string(),
                None,
                None,
                None,
                None,
                None,
                None,
            )
        }
        PortfolioEvent::Trade(trade) => {
            let sign = if trade.side == TradeSide::Buy {
                -rust_decimal::Decimal::ONE
            } else {
                rust_decimal::Decimal::ONE
            };
            let tax = trade.gross_amount_for_tax_in_eur()?;
            let rate = trade.tax_rate_to_eur.as_ref();
            (
                "Transação",
                format!(
                    "{} {}",
                    if trade.side == TradeSide::Buy {
                        "Compra"
                    } else {
                        "Venda"
                    },
                    instrument_label(&trade.instrument)
                ),
                (sign * trade.gross_amount().amount).to_string(),
                trade.unit_price.currency.code().to_string(),
                None,
                None,
                Some((sign * tax.amount).to_string()),
                rate.map(|value| value.rate.to_string()),
                rate.map(|value| value.reference_date),
                rate.map(|value| value.source.clone()),
            )
        }
        PortfolioEvent::CurrencyExchange(exchange) => (
            "Câmbio",
            format!(
                "{} {} → {} {}",
                exchange.sold.amount,
                exchange.sold.currency.code(),
                exchange.bought.amount,
                exchange.bought.currency.code()
            ),
            exchange.bought.amount.to_string(),
            exchange.bought.currency.code().to_string(),
            Some(exchange.sold.amount.to_string()),
            Some(exchange.sold.currency.code().to_string()),
            None,
            None,
            None,
            None,
        ),
        PortfolioEvent::Dividend(dividend) => {
            let net = dividend.net()?;
            (
                "Rendimento",
                format!("Dividendo {}", instrument_label(&dividend.instrument)),
                net.amount.to_string(),
                net.currency.code().to_string(),
                None,
                None,
                None,
                None,
                None,
                None,
            )
        }
    };
    let exchange_rate_label = match event {
        PortfolioEvent::Trade(trade) => trade.tax_rate_to_eur.as_ref().map(|rate| {
            format!(
                "1 EUR → {} {} · {}",
                (rust_decimal::Decimal::ONE / rate.rate)
                    .round_dp(6)
                    .normalize(),
                rate.base.code(),
                rate.source
            )
        }),
        PortfolioEvent::CurrencyExchange(exchange) => Some(format!(
            "1 {} → {} {} · {}",
            exchange.sold.currency.code(),
            (rust_decimal::Decimal::ONE / exchange.rate.rate)
                .round_dp(6)
                .normalize(),
            exchange.bought.currency.code(),
            exchange.rate.source
        )),
        _ => None,
    };
    Ok(PortfolioMovementOverview {
        id: event.id().to_string(),
        occurred_at: event.occurred_at(),
        kind: kind.to_string(),
        description,
        amount,
        currency,
        counter_amount,
        counter_currency,
        tax_amount_eur,
        tax_rate,
        tax_rate_date,
        tax_rate_source,
        exchange_rate_label,
    })
}

fn instrument_label(instrument: &Instrument) -> String {
    match instrument {
        Instrument::Equity { ticker } => ticker.clone(),
        Instrument::Option { occ_symbol } => occ_symbol.clone(),
    }
}
