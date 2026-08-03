# exchange-calendars-rs

A zero-overhead, production-grade global exchange calendar library for Rust, offering **O(1) lookups** for trading sessions, real-time market boundaries, and dynamic microstructure time-travel across 71 global markets.

## Key Features

- **Global Coverage**: 71 stock exchanges mapped natively (NYSE, BVMF, XMAD, XLON, XTKS, Crypto 24/7, and more).
- **Dual-Boundary Tracking**: Uses a high-density **5-byte daily bitmap** matrix to store both official regular session opening and closing bells natively.
- **Strict UTC Type-Safety**: Completely isolates and encapsules timezone gymnastics. The public API enforces absolute `DateTime<Utc>` signatures to prevent midnight calendar-rollover bugs.
- **Self-Healing Automation**: Integrated compile-time code generation (`build.rs`) monitors remote repository cache endpoints passively. It only updates and refreshes the binary tables if upstream holiday regulations change.
- **Data Pipeline Friendly**: Tailor-made for massive high-frequency calculations inside parallelized `Polars` DataFrames.

## Architecture Overview

The framework separates computational heavy-lifting from execution runtime:
1. **Python Codegen (`codegen/`)**: Interrogates Python's native `exchange_calendars` package using official `session_open` and `session_close` methods. It packs daily schedules into a compressed 5-byte block (1-byte status, 2-bytes open, 2-bytes close).
2. **Rust Meta-compilation (`build.rs`)**: Embeds the certficate of birth (4-byte header) and streams the compiled assets into static arrays at compilation time. No disk I/O at runtime.

## Public API Specification

```rust
pub trait ExchangeCalendar {
    fn mic(&self) -> &'static str;
    fn is_trading_day(&self, dt: &DateTime<Utc>) -> bool;
    fn open_time_on_date(&self, dt: &DateTime<Utc>) -> Option<DateTime<Utc>>;
    fn close_time_on_date(&self, dt: &DateTime<Utc>) -> Option<DateTime<Utc>>;
    fn is_open_at(&self, dt: &DateTime<Utc>) -> bool;
    fn calendar_bounds(&self) -> (DateTime<Utc>, DateTime<Utc>);
    fn latest_close_before(&self, dt: &DateTime<Utc>) -> Option<DateTime<Utc>>;
}
```

## Quick Start

```rust
use chrono::{TimeZone, Utc};
use exchange_calendars_rs::get_calendar;

fn main() {
    // Define an absolute universal snapshot (Wednesday afternoon)
    let analysis_instant = Utc.with_ymd_and_hms(2026, 7, 1, 16, 0, 0).unwrap();

    // Load London Stock Exchange
    let lse = get_calendar("XLON").unwrap();
    println!("Trading Day: {}", lse.is_trading_day(&analysis_instant));
    println!("Is Market Open: {}", lse.is_open_at(&analysis_instant));

    // Dynamic Previous Close Time-Travel (Safely accounts for intra-day open states)
    let bovespa = get_calendar("BVMF").unwrap();
    if let Some(prev_close) = bovespa.latest_close_before(&analysis_instant) {
        println!("Bovespa Latest Session Close: {} UTC", prev_close);
    }
}
```

