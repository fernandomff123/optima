use chrono::{TimeZone, Utc};
use exchange_calendars_rs::get_calendar;

fn main() {
    println!("--- CÁLCULO DE FECHO RECENTE (PREVIOUS CLOSE) ---");

    // Instante do teste: Domingo, 28 de Junho de 2026 às 16:00:00 UTC
    let domingo_utc = Utc.with_ymd_and_hms(2026, 7, 1, 16, 0, 0).unwrap();
    println!("Instante Atual do Robô: {}", domingo_utc);

    // 1. Testar Madrid (XMAD)
    let xmad = get_calendar("XMAD").unwrap();
    if let Some(ultimo_fecho) = xmad.latest_close_before(&domingo_utc) {
        println!("\nBolsa: {}", xmad.mic());
        // CORREÇÃO: Removemos o "UTC" manual. O ultimo_fecho (DateTime<Utc>) já imprime o fuso sozinho!
        println!("  -> Último Fecho Útil Encontrado: {}", ultimo_fecho);
    }

    // 2. Testar Bovespa (BVMF)
    let bvmf = get_calendar("BVMF").unwrap();
    if let Some(ultimo_fecho) = bvmf.latest_close_before(&domingo_utc) {
        println!("\nBolsa: {}", bvmf.mic());
        // CORREÇÃO: Usamos a variável real do escopo 'ultimo_fecho' sem duplicar o sufixo de texto
        println!("  -> Último Fecho Útil Encontrado: {}", ultimo_fecho);
    }
}
