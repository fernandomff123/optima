use super::{AssetOptionsReadModel, ContractDetail};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OptionKind {
    Call,
    Put,
}

impl OptionKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Call => "Call",
            Self::Put => "Put",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OptionQuote {
    Bid,
    Ask,
}

impl OptionQuote {
    pub fn label(self) -> &'static str {
        match self {
            Self::Bid => "Bid",
            Self::Ask => "Ask",
        }
    }

    pub fn action(self) -> &'static str {
        match self {
            Self::Bid => "SELL",
            Self::Ask => "BUY",
        }
    }

    pub fn position(self) -> &'static str {
        match self {
            Self::Bid => "SHORT",
            Self::Ask => "LONG",
        }
    }

    pub fn quantity(self) -> i32 {
        match self {
            Self::Bid => -1,
            Self::Ask => 1,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OptionSelection {
    pub row_index: usize,
    pub kind: OptionKind,
    pub quote: OptionQuote,
}

impl AssetOptionsReadModel {
    pub fn contract_for(&self, selection: OptionSelection) -> ContractDetail {
        let Some(row) = self.chain.get(selection.row_index) else {
            return self.contract.clone();
        };
        let side = match selection.kind {
            OptionKind::Call => &row.call,
            OptionKind::Put => &row.put,
        };
        let execution_price = match selection.quote {
            OptionQuote::Bid => &side.bid,
            OptionQuote::Ask => &side.ask,
        };
        let mut facts = self.contract.facts.clone();
        replace_fact(&mut facts, "Expiration", &self.expiration);
        replace_fact(&mut facts, "DTE", &self.dte);
        replace_fact(&mut facts, "Strike", &row.strike);
        replace_fact(&mut facts, "Type", selection.kind.label());
        ContractDetail {
            title: format!(
                "{} {} {} {}",
                self.symbol,
                self.expiration.to_uppercase(),
                row.strike,
                selection.kind.label().to_uppercase()
            ),
            price: execution_price.clone(),
            change: side.change.clone(),
            bid: side.bid.clone(),
            ask: side.ask.clone(),
            bid_size: side.bid_size.clone(),
            ask_size: side.ask_size.clone(),
            action: selection.quote.action().into(),
            position: selection.quote.position().into(),
            selected_quote: selection.quote.label().into(),
            quantity: selection.quote.quantity(),
            metrics: vec![
                ("Mid".into(), side.mid.clone()),
                ("Last Size".into(), side.last_size.clone()),
                ("Volume".into(), side.volume.clone()),
                ("Open Interest".into(), side.open_interest.clone()),
                ("IV".into(), format!("{}%", side.iv)),
                ("IV Rank".into(), self.iv_rank.clone()),
                ("Delta".into(), side.delta.clone()),
                ("Gamma".into(), side.gamma.clone()),
                ("Vega".into(), side.vega.clone()),
                ("Theta".into(), side.theta.clone()),
                ("Rho".into(), side.rho.clone()),
            ],
            facts,
        }
    }
}

fn replace_fact(facts: &mut [(String, String)], label: &str, value: &str) {
    if let Some((_, current)) = facts.iter_mut().find(|(candidate, _)| candidate == label) {
        *current = value.into();
    }
}
