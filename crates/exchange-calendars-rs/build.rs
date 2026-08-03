// build.rs
use std::fs;
use std::path::Path;

fn main() {
    let data_dir = Path::new("src/data");
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("generated_markets.rs");
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .unwrap()
        .replace("\\", "/");

    let mut match_arms = String::new();

    if data_dir.exists() {
        for entry in fs::read_dir(data_dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();

            if path.extension().map_or(false, |ext| ext == "bin") {
                let file_name = path.file_stem().unwrap().to_str().unwrap().to_uppercase();
                let file_path_rel = format!("src/data/{}.bin", file_name.to_lowercase());
                let user_mic = if file_name == "24_5" {
                    "24/5".to_string()
                } else if file_name == "24_7" {
                    "24/7".to_string()
                } else {
                    file_name.clone()
                };

                let tz_str = match user_mic.as_str() {
                    "XMAD" => "chrono_tz::Europe::Madrid",
                    "BVMF" => "chrono_tz::America::Sao_Paulo",
                    "XNYS" | "XCBO" => "chrono_tz::America::New_York",
                    "XLON" => "chrono_tz::Europe::London",
                    "XTKS" => "chrono_tz::Asia::Tokyo",
                    _ => "chrono_tz::UTC",
                };

                match_arms.push_str(&format!(
                    "        \"{}\" => Ok(Box::new(GenericCalendar::new(\"{}\", {}, include_bytes!(concat!(\"{}\", \"/{}\"))))),\n",
                                             user_mic, user_mic, tz_str, manifest_dir, file_path_rel
                ));
            }
        }
    }

    let generated_code = format!(
        "pub fn get_calendar(mic: &str) -> Result<Box<dyn ExchangeCalendar>, CalendarError> {{\n\
match mic.to_uppercase().as_str() {{\n\
{}\
_ => Err(CalendarError::MarketNotFound),\n\
}}\n\
}}",
        match_arms
    );

    fs::write(&dest_path, generated_code).unwrap();
    println!("cargo:rerun-if-changed=src/data");
}
