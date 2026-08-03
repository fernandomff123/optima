// src/lib.rs
use chrono::{DateTime, Duration, NaiveDate, NaiveTime, TimeZone, Utc};
use chrono_tz::Tz;

#[derive(Debug)]
pub enum CalendarError {
    MarketNotFound,
}

pub trait ExchangeCalendar {
    fn mic(&self) -> &'static str;
    fn is_trading_day(&self, dt: &DateTime<Utc>) -> bool;
    fn open_time_on_date(&self, dt: &DateTime<Utc>) -> Option<DateTime<Utc>>;
    fn close_time_on_date(&self, dt: &DateTime<Utc>) -> Option<DateTime<Utc>>;
    fn is_open_at(&self, dt: &DateTime<Utc>) -> bool;
    fn calendar_bounds(&self) -> (DateTime<Utc>, DateTime<Utc>);
    fn latest_close_before(&self, dt: &DateTime<Utc>) -> Option<DateTime<Utc>>;
}

pub struct GenericCalendar {
    mic: &'static str,
    tz: Tz,
    bytes: &'static [u8],
}

impl GenericCalendar {
    pub fn new(mic: &'static str, tz: Tz, bytes: &'static [u8]) -> Self {
        Self { mic, tz, bytes }
    }

    fn base_date(&self) -> NaiveDate {
        let mut ano_bytes = [0u8; 2];
        ano_bytes.copy_from_slice(&self.bytes[0..2]);
        let ano = u16::from_le_bytes(ano_bytes) as i32;
        let mes = self.bytes[2] as u32;
        let dia = self.bytes[3] as u32;
        NaiveDate::from_ymd_opt(ano, mes, dia).unwrap()
    }
}

impl ExchangeCalendar for GenericCalendar {
    fn mic(&self) -> &'static str {
        self.mic
    }

    fn calendar_bounds(&self) -> (DateTime<Utc>, DateTime<Utc>) {
        let start_date = self.base_date();
        let dados_len = self.bytes.len() - 4;
        let total_days = (dados_len / 5) as i64;
        let end_date = start_date + Duration::days(total_days - 1);

        let start_utc = self
            .tz
            .from_local_datetime(&start_date.and_hms_opt(0, 0, 0).unwrap())
            .earliest()
            .unwrap()
            .with_timezone(&Utc);
        let end_utc = self
            .tz
            .from_local_datetime(&end_date.and_hms_opt(23, 59, 59).unwrap())
            .earliest()
            .unwrap()
            .with_timezone(&Utc);
        (start_utc, end_utc)
    }

    fn is_trading_day(&self, dt: &DateTime<Utc>) -> bool {
        let dt_local = dt.with_timezone(&self.tz);
        let date_local = dt_local.date_naive();
        let start_date = self.base_date();
        let day_index = date_local.signed_duration_since(start_date).num_days();

        if day_index < 0 {
            return false;
        }
        let byte_offset = 4 + (day_index as usize) * 5;
        if byte_offset >= self.bytes.len() {
            return false;
        }

        let day_status = self.bytes[byte_offset];
        day_status == 1 || day_status == 2
    }

    fn open_time_on_date(&self, dt: &DateTime<Utc>) -> Option<DateTime<Utc>> {
        let dt_local = dt.with_timezone(&self.tz);
        let date_local = dt_local.date_naive();
        let start_date = self.base_date();
        let day_index = date_local.signed_duration_since(start_date).num_days();

        if day_index < 0 {
            return None;
        }
        let byte_offset = 4 + (day_index as usize) * 5;
        if byte_offset >= self.bytes.len() || self.bytes[byte_offset] == 0 {
            return None;
        }

        let min_b1 = self.bytes[byte_offset + 1] as u16;
        let min_b2 = self.bytes[byte_offset + 2] as u16;
        let minutos = min_b1 | (min_b2 << 8);

        let hora_local = NaiveTime::from_hms_opt((minutos / 60) as u32, (minutos % 60) as u32, 0)?;
        let dt_local_rebuilt = date_local.and_time(hora_local);

        match self.tz.from_local_datetime(&dt_local_rebuilt) {
            chrono::LocalResult::Single(t) => Some(t.with_timezone(&Utc)),
            chrono::LocalResult::Ambiguous(t1, _) => Some(t1.with_timezone(&Utc)),
            chrono::LocalResult::None => None,
        }
    }

    fn close_time_on_date(&self, dt: &DateTime<Utc>) -> Option<DateTime<Utc>> {
        let dt_local = dt.with_timezone(&self.tz);
        let date_local = dt_local.date_naive();
        let start_date = self.base_date();
        let day_index = date_local.signed_duration_since(start_date).num_days();

        if day_index < 0 {
            return None;
        }
        let byte_offset = 4 + (day_index as usize) * 5;
        if byte_offset >= self.bytes.len() || self.bytes[byte_offset] == 0 {
            return None;
        }

        let min_b1 = self.bytes[byte_offset + 3] as u16;
        let min_b2 = self.bytes[byte_offset + 4] as u16;
        let minutos = min_b1 | (min_b2 << 8);

        let hora_local = NaiveTime::from_hms_opt((minutos / 60) as u32, (minutos % 60) as u32, 0)?;
        let dt_local_rebuilt = date_local.and_time(hora_local);

        match self.tz.from_local_datetime(&dt_local_rebuilt) {
            chrono::LocalResult::Single(t) => Some(t.with_timezone(&Utc)),
            chrono::LocalResult::Ambiguous(t1, _) => Some(t1.with_timezone(&Utc)),
            chrono::LocalResult::None => None,
        }
    }

    fn is_open_at(&self, dt: &DateTime<Utc>) -> bool {
        if !self.is_trading_day(dt) {
            return false;
        }

        let open_utc = self.open_time_on_date(dt);
        let close_utc = self.close_time_on_date(dt);

        if let (Some(open), Some(close)) = (open_utc, close_utc) {
            dt >= &open && dt <= &close
        } else {
            false
        }
    }

    fn latest_close_before(&self, dt: &DateTime<Utc>) -> Option<DateTime<Utc>> {
        let dt_local = dt.with_timezone(&self.tz);
        let mut data_analise = dt_local.date_naive();
        let start_date = self.base_date();

        while data_analise >= start_date {
            let day_index = data_analise.signed_duration_since(start_date).num_days();
            let byte_offset = 4 + (day_index as usize) * 5;

            if byte_offset >= self.bytes.len() {
                data_analise = data_analise - Duration::days(1);
                continue;
            }

            if self.bytes[byte_offset] == 1 || self.bytes[byte_offset] == 2 {
                let dt_intermediario = self
                    .tz
                    .from_local_datetime(&data_analise.and_hms_opt(12, 0, 0).unwrap())
                    .earliest()?
                    .with_timezone(&Utc);

                if let Some(fecho_utc) = self.close_time_on_date(&dt_intermediario) {
                    if &fecho_utc > dt {
                        data_analise = data_analise - Duration::days(1);
                        continue;
                    }
                    return Some(fecho_utc);
                }
            }
            data_analise = data_analise - Duration::days(1);
        }
        None
    }
}

// 🧠 A FUNÇÃO MESTRE FICA AQUI FORA DE TODOS OS BLOCOS STRUCT/TRAIT:
pub fn parse_to_utc(
    input_str: &str,
    format: Option<&str>,
) -> Result<DateTime<Utc>, chrono::ParseError> {
    if let Some(fmt) = format {
        if let Ok(naive_dt) = chrono::NaiveDateTime::parse_from_str(input_str, fmt) {
            return Ok(Utc.from_utc_datetime(&naive_dt));
        }
        let naive_date = NaiveDate::parse_from_str(input_str, fmt)?;
        return Ok(Utc.from_utc_datetime(&naive_date.and_hms_opt(0, 0, 0).unwrap()));
    }

    if input_str.len() == 10 {
        let naive_date = NaiveDate::parse_from_str(input_str, "%Y-%m-%d")?;
        Ok(Utc.from_utc_datetime(&naive_date.and_hms_opt(0, 0, 0).unwrap()))
    } else {
        let naive_dt = chrono::NaiveDateTime::parse_from_str(input_str, "%Y-%m-%d %H:%M:%S")?;
        Ok(Utc.from_utc_datetime(&naive_dt))
    }
}

// 🧠 A INJEÇÃO AUTOMÁTICA DAS 71 BOLSAS DO BUILD.RS FICA MESMO NO FIM DE TUDO:
include!(concat!(env!("OUT_DIR"), "/generated_markets.rs"));
