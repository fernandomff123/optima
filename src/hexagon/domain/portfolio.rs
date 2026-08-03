use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, error::Error, fmt, str::FromStr};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Currency(String);

impl Currency {
    pub fn new(code: &str) -> Result<Self, PortfolioError> {
        let code = code.trim().to_ascii_uppercase();
        if code.len() != 3 || !code.bytes().all(|value| value.is_ascii_alphabetic()) {
            return Err(PortfolioError::InvalidCurrency(code));
        }
        Ok(Self(code))
    }

    pub fn eur() -> Self {
        Self("EUR".to_string())
    }

    pub fn code(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Money {
    pub amount: Decimal,
    pub currency: Currency,
}

impl Money {
    pub fn new(amount: Decimal, currency: Currency) -> Self {
        Self { amount, currency }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExchangeRate {
    pub base: Currency,
    pub quote: Currency,
    pub rate: Decimal,
    pub reference_date: NaiveDate,
    pub source: String,
}

impl ExchangeRate {
    pub fn new(
        base: Currency,
        quote: Currency,
        rate: Decimal,
        reference_date: NaiveDate,
        source: impl Into<String>,
    ) -> Result<Self, PortfolioError> {
        let source = source.into();
        if rate <= Decimal::ZERO {
            return Err(PortfolioError::InvalidExchangeRate);
        }
        if source.trim().is_empty() {
            return Err(PortfolioError::MissingExchangeRateSource);
        }
        Ok(Self {
            base,
            quote,
            rate,
            reference_date,
            source,
        })
    }

    pub fn convert(&self, money: &Money) -> Result<Money, PortfolioError> {
        if money.currency != self.base {
            return Err(PortfolioError::ExchangeRateCurrencyMismatch);
        }
        Ok(Money::new(money.amount * self.rate, self.quote.clone()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Instrument {
    Equity { ticker: String },
    Option { occ_symbol: String },
}

impl Instrument {
    pub fn contract_multiplier(&self) -> Decimal {
        match self {
            Self::Equity { .. } => Decimal::ONE,
            Self::Option { .. } => Decimal::ONE_HUNDRED,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CashMovementKind {
    Deposit,
    Withdrawal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CashMovement {
    pub id: String,
    pub occurred_at: DateTime<Utc>,
    pub kind: CashMovementKind,
    pub amount: Money,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CurrencyExchange {
    pub id: String,
    pub occurred_at: DateTime<Utc>,
    pub sold: Money,
    pub bought: Money,
    pub rate: ExchangeRate,
}

impl CurrencyExchange {
    pub fn new(
        id: impl Into<String>,
        occurred_at: DateTime<Utc>,
        sold: Money,
        bought: Money,
        rate: ExchangeRate,
    ) -> Result<Self, PortfolioError> {
        let id = id.into();
        if id.trim().is_empty() {
            return Err(PortfolioError::MissingTransactionId);
        }
        if sold.amount <= Decimal::ZERO || bought.amount <= Decimal::ZERO {
            return Err(PortfolioError::InvalidAmount);
        }
        if rate.base != bought.currency || rate.quote != sold.currency {
            return Err(PortfolioError::ExchangeRateCurrencyMismatch);
        }
        if rate.convert(&bought)?.amount != sold.amount {
            return Err(PortfolioError::ExchangeAmountsMismatch);
        }
        Ok(Self {
            id,
            occurred_at,
            sold,
            bought,
            rate,
        })
    }
}

impl CashMovement {
    pub fn new(
        id: impl Into<String>,
        occurred_at: DateTime<Utc>,
        kind: CashMovementKind,
        amount: Money,
    ) -> Result<Self, PortfolioError> {
        let id = id.into();
        if id.trim().is_empty() {
            return Err(PortfolioError::MissingTransactionId);
        }
        if amount.amount <= Decimal::ZERO {
            return Err(PortfolioError::InvalidAmount);
        }
        Ok(Self {
            id,
            occurred_at,
            kind,
            amount,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dividend {
    pub id: String,
    pub instrument: Instrument,
    pub paid_at: DateTime<Utc>,
    pub gross: Money,
    pub withholding_tax: Money,
    pub tax_rate_to_eur: Option<ExchangeRate>,
}

impl Dividend {
    pub fn net(&self) -> Result<Money, PortfolioError> {
        if self.gross.currency != self.withholding_tax.currency
            || self.withholding_tax.amount < Decimal::ZERO
            || self.withholding_tax.amount > self.gross.amount
        {
            return Err(PortfolioError::InvalidWithholdingTax);
        }
        Ok(Money::new(
            self.gross.amount - self.withholding_tax.amount,
            self.gross.currency.clone(),
        ))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TradeSide {
    Buy,
    Sell,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Trade {
    pub id: String,
    pub instrument: Instrument,
    pub side: TradeSide,
    pub executed_at: DateTime<Utc>,
    pub quantity: Decimal,
    pub unit_price: Money,
    pub fees: Vec<Money>,
    pub settlement_rate_to_eur: Option<ExchangeRate>,
    pub tax_rate_to_eur: Option<ExchangeRate>,
}

impl Trade {
    pub fn new(
        id: impl Into<String>,
        instrument: Instrument,
        side: TradeSide,
        executed_at: DateTime<Utc>,
        quantity: Decimal,
        unit_price: Money,
    ) -> Result<Self, PortfolioError> {
        let id = id.into();
        if id.trim().is_empty() {
            return Err(PortfolioError::MissingTransactionId);
        }
        if quantity <= Decimal::ZERO {
            return Err(PortfolioError::InvalidQuantity);
        }
        if unit_price.amount < Decimal::ZERO {
            return Err(PortfolioError::InvalidPrice);
        }
        Ok(Self {
            id,
            instrument,
            side,
            executed_at,
            quantity,
            unit_price,
            fees: Vec::new(),
            settlement_rate_to_eur: None,
            tax_rate_to_eur: None,
        })
    }

    pub fn gross_amount(&self) -> Money {
        Money::new(
            self.quantity * self.instrument.contract_multiplier() * self.unit_price.amount,
            self.unit_price.currency.clone(),
        )
    }

    pub fn gross_amount_for_tax_in_eur(&self) -> Result<Money, PortfolioError> {
        self.money_for_tax_in_eur(&self.gross_amount())
    }

    fn money_for_tax_in_eur(&self, money: &Money) -> Result<Money, PortfolioError> {
        if money.currency == Currency::eur() {
            return Ok(money.clone());
        }
        self.tax_rate_to_eur
            .as_ref()
            .ok_or(PortfolioError::MissingTaxExchangeRate)?
            .convert(money)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PortfolioEvent {
    Trade(Trade),
    CashMovement(CashMovement),
    Dividend(Dividend),
    CurrencyExchange(CurrencyExchange),
}

impl PortfolioEvent {
    pub fn id(&self) -> &str {
        match self {
            Self::Trade(v) => &v.id,
            Self::CashMovement(v) => &v.id,
            Self::CurrencyExchange(v) => &v.id,
            Self::Dividend(v) => &v.id,
        }
    }

    pub fn occurred_at(&self) -> DateTime<Utc> {
        match self {
            Self::Trade(value) => value.executed_at,
            Self::CashMovement(value) => value.occurred_at,
            Self::CurrencyExchange(value) => value.occurred_at,
            Self::Dividend(value) => value.paid_at,
        }
    }

    pub fn kind_name(&self) -> &'static str {
        match self {
            Self::Trade(_) => "trade",
            Self::CashMovement(_) => "cash_movement",
            Self::CurrencyExchange(_) => "currency_exchange",
            Self::Dividend(_) => "dividend",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Position {
    pub instrument: Instrument,
    pub quantity: Decimal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaxLot {
    pub acquisition_trade_id: String,
    pub instrument: Instrument,
    pub acquired_at: DateTime<Utc>,
    pub remaining_quantity: Decimal,
    pub unit_cost_eur: Decimal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RealizedLot {
    pub acquisition_trade_id: String,
    pub disposal_trade_id: String,
    pub instrument: Instrument,
    pub quantity: Decimal,
    pub acquisition_cost_eur: Decimal,
    pub disposal_proceeds_eur: Decimal,
    pub gain_eur: Decimal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FifoTaxProjection {
    pub open_lots: Vec<TaxLot>,
    pub realized: Vec<RealizedLot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Portfolio {
    pub id: String,
    pub name: String,
    pub base_currency: Currency,
    events: Vec<PortfolioEvent>,
}

impl Portfolio {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        base_currency: Currency,
    ) -> Result<Self, PortfolioError> {
        let (id, name) = (id.into(), name.into());
        if id.trim().is_empty() || name.trim().is_empty() {
            return Err(PortfolioError::InvalidPortfolioIdentity);
        }
        Ok(Self {
            id,
            name,
            base_currency,
            events: Vec::new(),
        })
    }

    pub fn record(&mut self, event: PortfolioEvent) -> Result<(), PortfolioError> {
        if self.events.iter().any(|stored| stored.id() == event.id()) {
            return Err(PortfolioError::DuplicateEventId(event.id().to_string()));
        }
        self.events.push(event);
        Ok(())
    }

    pub fn events(&self) -> &[PortfolioEvent] {
        &self.events
    }

    pub fn positions(&self) -> Vec<Position> {
        let mut values = BTreeMap::<Instrument, Decimal>::new();
        for event in &self.events {
            if let PortfolioEvent::Trade(trade) = event {
                let sign = if trade.side == TradeSide::Buy {
                    Decimal::ONE
                } else {
                    -Decimal::ONE
                };
                *values.entry(trade.instrument.clone()).or_default() += sign * trade.quantity;
            }
        }
        values
            .into_iter()
            .filter(|(_, quantity)| !quantity.is_zero())
            .map(|(instrument, quantity)| Position {
                instrument,
                quantity,
            })
            .collect()
    }

    pub fn cash_balances(&self) -> BTreeMap<String, Decimal> {
        let mut values = BTreeMap::new();
        for event in &self.events {
            match event {
                PortfolioEvent::CashMovement(v) => add_money(
                    &mut values,
                    &v.amount,
                    if v.kind == CashMovementKind::Deposit {
                        Decimal::ONE
                    } else {
                        -Decimal::ONE
                    },
                ),
                PortfolioEvent::Trade(v) => {
                    add_money(
                        &mut values,
                        &v.gross_amount(),
                        if v.side == TradeSide::Buy {
                            -Decimal::ONE
                        } else {
                            Decimal::ONE
                        },
                    );
                    for fee in &v.fees {
                        add_money(&mut values, fee, -Decimal::ONE);
                    }
                }
                PortfolioEvent::CurrencyExchange(v) => {
                    add_money(&mut values, &v.sold, -Decimal::ONE);
                    add_money(&mut values, &v.bought, Decimal::ONE);
                }
                PortfolioEvent::Dividend(v) => {
                    if let Ok(net) = v.net() {
                        add_money(&mut values, &net, Decimal::ONE);
                    }
                }
            }
        }
        values
    }

    pub fn cash_value_in_base(&self) -> Result<Money, PortfolioError> {
        let mut rates = BTreeMap::<String, &ExchangeRate>::new();
        for event in &self.events {
            let rate = match event {
                PortfolioEvent::Trade(trade) => trade
                    .settlement_rate_to_eur
                    .as_ref()
                    .or(trade.tax_rate_to_eur.as_ref()),
                PortfolioEvent::CurrencyExchange(exchange) => Some(&exchange.rate),
                PortfolioEvent::Dividend(dividend) => dividend.tax_rate_to_eur.as_ref(),
                PortfolioEvent::CashMovement(_) => None,
            };
            if let Some(rate) = rate
                && rate.quote == self.base_currency
                && rates
                    .get(rate.base.code())
                    .is_none_or(|stored| stored.reference_date <= rate.reference_date)
            {
                rates.insert(rate.base.code().to_string(), rate);
            }
        }
        let mut total = Decimal::ZERO;
        for (currency, amount) in self.cash_balances() {
            if currency == self.base_currency.code() {
                total += amount;
            } else {
                let rate = rates.get(&currency).ok_or_else(|| {
                    PortfolioError::MissingValuationExchangeRate(currency.clone())
                })?;
                total += amount * rate.rate;
            }
        }
        Ok(Money::new(total, self.base_currency.clone()))
    }

    pub fn fifo_tax_projection(&self) -> Result<FifoTaxProjection, PortfolioError> {
        let mut trades = self
            .events
            .iter()
            .filter_map(|event| match event {
                PortfolioEvent::Trade(trade) => Some(trade),
                _ => None,
            })
            .collect::<Vec<_>>();
        trades.sort_by_key(|trade| trade.executed_at);
        let mut lots = BTreeMap::<Instrument, Vec<TaxLot>>::new();
        let mut realized = Vec::new();
        for trade in trades {
            let fees_eur = trade.fees.iter().try_fold(Decimal::ZERO, |total, fee| {
                Ok::<_, PortfolioError>(total + trade.money_for_tax_in_eur(fee)?.amount)
            })?;
            let gross_eur = trade.gross_amount_for_tax_in_eur()?.amount;
            match trade.side {
                TradeSide::Buy => lots
                    .entry(trade.instrument.clone())
                    .or_default()
                    .push(TaxLot {
                        acquisition_trade_id: trade.id.clone(),
                        instrument: trade.instrument.clone(),
                        acquired_at: trade.executed_at,
                        remaining_quantity: trade.quantity,
                        unit_cost_eur: (gross_eur + fees_eur) / trade.quantity,
                    }),
                TradeSide::Sell => {
                    let unit_proceeds = (gross_eur - fees_eur) / trade.quantity;
                    let instrument_lots = lots.entry(trade.instrument.clone()).or_default();
                    let available: Decimal = instrument_lots
                        .iter()
                        .map(|lot| lot.remaining_quantity)
                        .sum();
                    if available < trade.quantity {
                        return Err(PortfolioError::UnsupportedShortDisposal(trade.id.clone()));
                    }
                    let mut remaining = trade.quantity;
                    for lot in instrument_lots
                        .iter_mut()
                        .filter(|lot| !lot.remaining_quantity.is_zero())
                    {
                        if remaining.is_zero() {
                            break;
                        }
                        let matched = remaining.min(lot.remaining_quantity);
                        let cost = matched * lot.unit_cost_eur;
                        let proceeds = matched * unit_proceeds;
                        realized.push(RealizedLot {
                            acquisition_trade_id: lot.acquisition_trade_id.clone(),
                            disposal_trade_id: trade.id.clone(),
                            instrument: trade.instrument.clone(),
                            quantity: matched,
                            acquisition_cost_eur: cost,
                            disposal_proceeds_eur: proceeds,
                            gain_eur: proceeds - cost,
                        });
                        lot.remaining_quantity -= matched;
                        remaining -= matched;
                    }
                }
            }
        }
        Ok(FifoTaxProjection {
            open_lots: lots
                .into_values()
                .flatten()
                .filter(|lot| !lot.remaining_quantity.is_zero())
                .collect(),
            realized,
        })
    }

    pub fn realized_gains_by_currency(&self) -> Result<BTreeMap<String, Decimal>, PortfolioError> {
        let mut trades = self
            .events
            .iter()
            .filter_map(|event| match event {
                PortfolioEvent::Trade(trade) => Some(trade),
                _ => None,
            })
            .collect::<Vec<_>>();
        trades.sort_by_key(|trade| trade.executed_at);
        let mut lots = BTreeMap::<Instrument, Vec<(Decimal, Decimal, Currency)>>::new();
        let mut realized = BTreeMap::<String, Decimal>::new();
        for trade in trades {
            let currency = trade.unit_price.currency.clone();
            let fees = trade.fees.iter().try_fold(Decimal::ZERO, |total, fee| {
                if fee.currency != currency {
                    return Err(PortfolioError::MixedTradeCurrencies);
                }
                Ok(total + fee.amount)
            })?;
            let gross = trade.gross_amount().amount;
            match trade.side {
                TradeSide::Buy => lots.entry(trade.instrument.clone()).or_default().push((
                    trade.quantity,
                    (gross + fees) / trade.quantity,
                    currency,
                )),
                TradeSide::Sell => {
                    let instrument_lots = lots.entry(trade.instrument.clone()).or_default();
                    let available = instrument_lots
                        .iter()
                        .map(|(quantity, _, _)| *quantity)
                        .sum::<Decimal>();
                    if available < trade.quantity {
                        return Err(PortfolioError::UnsupportedShortDisposal(trade.id.clone()));
                    }
                    let unit_proceeds = (gross - fees) / trade.quantity;
                    let mut remaining = trade.quantity;
                    for (lot_quantity, unit_cost, lot_currency) in instrument_lots
                        .iter_mut()
                        .filter(|(quantity, _, _)| !quantity.is_zero())
                    {
                        if remaining.is_zero() {
                            break;
                        }
                        if *lot_currency != currency {
                            return Err(PortfolioError::MixedTradeCurrencies);
                        }
                        let matched = remaining.min(*lot_quantity);
                        *realized.entry(currency.code().to_string()).or_default() +=
                            matched * (unit_proceeds - *unit_cost);
                        *lot_quantity -= matched;
                        remaining -= matched;
                    }
                }
            }
        }
        Ok(realized)
    }
}

fn add_money(values: &mut BTreeMap<String, Decimal>, money: &Money, sign: Decimal) {
    *values.entry(money.currency.code().to_string()).or_default() += sign * money.amount;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PortfolioError {
    InvalidCurrency(String),
    InvalidExchangeRate,
    MissingExchangeRateSource,
    ExchangeRateCurrencyMismatch,
    ExchangeAmountsMismatch,
    MissingTransactionId,
    InvalidQuantity,
    InvalidPrice,
    InvalidAmount,
    MissingTaxExchangeRate,
    InvalidWithholdingTax,
    InvalidPortfolioIdentity,
    DuplicateEventId(String),
    UnsupportedShortDisposal(String),
    MixedTradeCurrencies,
    MissingValuationExchangeRate(String),
}

impl fmt::Display for PortfolioError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCurrency(code) => write!(formatter, "moeda ISO inválida: {code}"),
            Self::InvalidExchangeRate => write!(formatter, "a taxa de câmbio deve ser positiva"),
            Self::MissingExchangeRateSource => {
                write!(formatter, "a taxa de câmbio requer uma fonte")
            }
            Self::ExchangeRateCurrencyMismatch => write!(
                formatter,
                "a moeda não corresponde à base da taxa de câmbio"
            ),
            Self::ExchangeAmountsMismatch => {
                write!(formatter, "os montantes não correspondem à taxa de câmbio")
            }
            Self::MissingTransactionId => write!(formatter, "a transação requer um identificador"),
            Self::InvalidQuantity => write!(formatter, "a quantidade deve ser positiva"),
            Self::InvalidPrice => write!(formatter, "o preço não pode ser negativo"),
            Self::InvalidAmount => write!(formatter, "o montante deve ser positivo"),
            Self::MissingTaxExchangeRate => write!(
                formatter,
                "falta a taxa histórica para conversão fiscal em EUR"
            ),
            Self::InvalidWithholdingTax => write!(formatter, "retenção de dividendo inválida"),
            Self::InvalidPortfolioIdentity => write!(formatter, "portfolio requer id e nome"),
            Self::DuplicateEventId(id) => write!(formatter, "evento duplicado: {id}"),
            Self::UnsupportedShortDisposal(id) => {
                write!(formatter, "a venda {id} excede os lotes longos disponíveis")
            }
            Self::MixedTradeCurrencies => {
                write!(
                    formatter,
                    "lote, transação e comissões devem usar a mesma moeda"
                )
            }
            Self::MissingValuationExchangeRate(currency) => {
                write!(
                    formatter,
                    "falta uma taxa para valorizar o saldo em {currency}"
                )
            }
        }
    }
}

impl Error for PortfolioError {}

pub fn decimal(value: &str) -> Result<Decimal, rust_decimal::Error> {
    Decimal::from_str(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn usd_trade() -> Trade {
        Trade::new(
            "trade-1",
            Instrument::Equity {
                ticker: "AAPL".to_string(),
            },
            TradeSide::Buy,
            Utc.with_ymd_and_hms(2026, 7, 17, 14, 30, 0).unwrap(),
            decimal("10").unwrap(),
            Money::new(decimal("200.15").unwrap(), Currency::new("usd").unwrap()),
        )
        .unwrap()
    }

    #[test]
    fn preserves_original_money_and_uses_the_recorded_tax_rate() {
        let mut trade = usd_trade();
        trade.tax_rate_to_eur = Some(
            ExchangeRate::new(
                Currency::new("USD").unwrap(),
                Currency::eur(),
                decimal("0.92").unwrap(),
                NaiveDate::from_ymd_opt(2026, 7, 17).unwrap(),
                "BCE",
            )
            .unwrap(),
        );

        assert_eq!(trade.gross_amount().amount, decimal("2001.50").unwrap());
        assert_eq!(
            trade.gross_amount_for_tax_in_eur().unwrap().amount,
            decimal("1841.3800").unwrap()
        );
        assert_eq!(trade.unit_price.currency.code(), "USD");
    }

    #[test]
    fn applies_the_standard_option_contract_multiplier() {
        let trade = Trade::new(
            "option-1",
            Instrument::Option {
                occ_symbol: "IBM260814C00220000".to_string(),
            },
            TradeSide::Buy,
            Utc.with_ymd_and_hms(2026, 7, 16, 20, 56, 30).unwrap(),
            Decimal::ONE,
            Money::new(decimal("12.025").unwrap(), Currency::new("USD").unwrap()),
        )
        .unwrap();

        assert_eq!(trade.gross_amount().amount, decimal("1202.5").unwrap());
    }

    #[test]
    fn currency_exchange_moves_both_original_currency_balances() {
        let mut portfolio = Portfolio::new("main", "Principal", Currency::eur()).unwrap();
        let usd = Currency::new("USD").unwrap();
        portfolio
            .record(PortfolioEvent::CurrencyExchange(
                CurrencyExchange::new(
                    "fx-1",
                    Utc.with_ymd_and_hms(2026, 7, 16, 20, 56, 30).unwrap(),
                    Money::new(decimal("2156.45").unwrap(), Currency::eur()),
                    Money::new(decimal("2507.5").unwrap(), usd.clone()),
                    ExchangeRate::new(
                        usd,
                        Currency::eur(),
                        decimal("0.86").unwrap(),
                        NaiveDate::from_ymd_opt(2026, 7, 16).unwrap(),
                        "TEST",
                    )
                    .unwrap(),
                )
                .unwrap(),
            ))
            .unwrap();

        let cash = portfolio.cash_balances();
        assert_eq!(cash["EUR"], decimal("-2156.45").unwrap());
        assert_eq!(cash["USD"], decimal("2507.5").unwrap());
        assert_eq!(
            portfolio.cash_value_in_base().unwrap().amount,
            Decimal::ZERO
        );
    }

    #[test]
    fn refuses_to_infer_a_missing_historical_tax_rate() {
        assert_eq!(
            usd_trade().gross_amount_for_tax_in_eur(),
            Err(PortfolioError::MissingTaxExchangeRate)
        );
    }

    #[test]
    fn rebuilds_positions_and_multicurrency_cash_from_events() {
        let mut portfolio = Portfolio::new("main", "Principal", Currency::eur()).unwrap();
        portfolio
            .record(PortfolioEvent::CashMovement(CashMovement {
                id: "deposit-1".to_string(),
                occurred_at: Utc.with_ymd_and_hms(2026, 7, 17, 8, 0, 0).unwrap(),
                kind: CashMovementKind::Deposit,
                amount: Money::new(decimal("5000").unwrap(), Currency::new("USD").unwrap()),
            }))
            .unwrap();
        let mut trade = usd_trade();
        trade.fees.push(Money::new(
            decimal("1.25").unwrap(),
            Currency::new("USD").unwrap(),
        ));
        portfolio
            .record(PortfolioEvent::Trade(trade.clone()))
            .unwrap();

        assert_eq!(
            portfolio.positions(),
            vec![Position {
                instrument: trade.instrument.clone(),
                quantity: decimal("10").unwrap()
            }]
        );
        assert_eq!(
            portfolio.cash_balances()["USD"],
            decimal("2997.25").unwrap()
        );
        assert_eq!(
            portfolio.record(PortfolioEvent::Trade(trade)),
            Err(PortfolioError::DuplicateEventId("trade-1".to_string()))
        );
    }

    #[test]
    fn matches_disposals_to_fifo_lots_in_euros() {
        let mut portfolio = Portfolio::new("main", "Principal", Currency::eur()).unwrap();
        let instrument = Instrument::Equity {
            ticker: "AAPL".to_string(),
        };
        let rate = |date| {
            ExchangeRate::new(
                Currency::new("USD").unwrap(),
                Currency::eur(),
                decimal("0.9").unwrap(),
                date,
                "BCE",
            )
            .unwrap()
        };
        for (id, day, quantity, price, side) in [
            ("buy-1", 1, "10", "100", TradeSide::Buy),
            ("buy-2", 2, "5", "120", TradeSide::Buy),
            ("sell-1", 3, "12", "150", TradeSide::Sell),
        ] {
            let mut trade = Trade::new(
                id,
                instrument.clone(),
                side,
                Utc.with_ymd_and_hms(2026, 7, day, 14, 30, 0).unwrap(),
                decimal(quantity).unwrap(),
                Money::new(decimal(price).unwrap(), Currency::new("USD").unwrap()),
            )
            .unwrap();
            trade.tax_rate_to_eur = Some(rate(NaiveDate::from_ymd_opt(2026, 7, day).unwrap()));
            portfolio.record(PortfolioEvent::Trade(trade)).unwrap();
        }

        let projection = portfolio.fifo_tax_projection().unwrap();
        assert_eq!(projection.realized.len(), 2);
        assert_eq!(projection.realized[0].gain_eur, decimal("450.0").unwrap());
        assert_eq!(projection.realized[1].gain_eur, decimal("54.0").unwrap());
        assert_eq!(
            projection.open_lots[0].remaining_quantity,
            decimal("3").unwrap()
        );
        assert_eq!(projection.open_lots[0].acquisition_trade_id, "buy-2");
        assert_eq!(
            portfolio.realized_gains_by_currency().unwrap()["USD"],
            decimal("560").unwrap()
        );
    }
}
