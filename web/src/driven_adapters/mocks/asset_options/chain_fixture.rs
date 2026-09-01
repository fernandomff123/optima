use crate::ports::asset_options::{OptionChainRowSnapshot, OptionSideSnapshot};

use super::details::{CALL_DETAILS, PUT_DETAILS};

fn side(values: [&'static str; 7], details: [&'static str; 9]) -> OptionSideSnapshot {
    OptionSideSnapshot {
        last: values[0],
        change: details[0],
        bid: values[1],
        ask: values[2],
        mid: details[1],
        bid_size: details[2],
        ask_size: details[3],
        last_size: details[4],
        iv: values[3],
        delta: values[4],
        gamma: details[5],
        vega: details[6],
        theta: details[7],
        rho: details[8],
        open_interest: values[5],
        volume: values[6],
    }
}

fn row(
    index: usize,
    strike: &'static str,
    call: [&'static str; 7],
    put: [&'static str; 7],
    is_atm: bool,
    is_selected: bool,
) -> OptionChainRowSnapshot {
    OptionChainRowSnapshot {
        strike,
        is_atm,
        is_selected,
        call: side(call, CALL_DETAILS[index]),
        put: side(put, PUT_DETAILS[index]),
    }
}

pub(super) fn chain() -> Vec<OptionChainRowSnapshot> {
    vec![
        row(
            0,
            "180.00",
            ["12.85", "12.60", "13.10", "24.1", "0.83", "8,432", "3,287"],
            ["0.33", "0.32", "0.35", "24.7", "-0.17", "12,356", "2,914"],
            false,
            false,
        ),
        row(
            1,
            "182.50",
            ["9.80", "9.60", "10.00", "23.7", "0.78", "9,215", "3,671"],
            ["0.46", "0.45", "0.48", "24.1", "-0.22", "13,284", "3,126"],
            false,
            false,
        ),
        row(
            2,
            "185.00",
            ["7.30", "7.15", "7.45", "23.4", "0.72", "10,134", "4,025"],
            ["0.64", "0.63", "0.66", "23.6", "-0.28", "14,902", "3,580"],
            false,
            false,
        ),
        row(
            3,
            "187.50",
            ["5.35", "5.20", "5.50", "23.0", "0.65", "11,278", "4,832"],
            ["0.88", "0.87", "0.91", "23.1", "-0.35", "16,278", "4,213"],
            false,
            false,
        ),
        row(
            4,
            "190.00",
            ["3.85", "3.75", "3.95", "22.6", "0.58", "12,562", "5,249"],
            ["1.23", "1.21", "1.26", "22.6", "-0.42", "17,642", "4,812"],
            false,
            false,
        ),
        row(
            5,
            "192.50",
            ["2.70", "2.61", "2.79", "22.2", "0.50", "13,845", "6,012"],
            ["1.70", "1.67", "1.74", "22.2", "-0.50", "18,954", "5,612"],
            true,
            false,
        ),
        row(
            6,
            "195.00",
            ["1.82", "1.74", "1.90", "21.9", "0.42", "14,932", "6,401"],
            ["2.32", "2.28", "2.36", "21.9", "-0.58", "19,876", "6,203"],
            false,
            false,
        ),
        row(
            7,
            "197.50",
            ["1.15", "1.08", "1.22", "21.6", "0.33", "16,274", "7,318"],
            ["3.10", "3.05", "3.15", "21.6", "-0.67", "20,485", "6,157"],
            false,
            true,
        ),
        row(
            8,
            "200.00",
            ["0.68", "0.63", "0.72", "21.4", "0.24", "17,112", "6,582"],
            ["4.05", "4.00", "4.12", "21.4", "-0.76", "21,177", "5,241"],
            false,
            false,
        ),
        row(
            9,
            "202.50",
            ["0.38", "0.34", "0.41", "21.2", "0.16", "18,065", "5,404"],
            ["5.20", "5.12", "5.28", "21.2", "-0.84", "21,568", "4,312"],
            false,
            false,
        ),
        row(
            10,
            "205.00",
            ["0.20", "0.17", "0.22", "21.1", "0.10", "18,943", "3,762"],
            ["6.55", "6.45", "6.66", "21.1", "-0.90", "21,943", "3,284"],
            false,
            false,
        ),
    ]
}
