
import exchange_calendars as xcals
import pandas as pd
import struct
import os

def export_all_markets():
    all_mics = sorted(xcals.get_calendar_names(include_aliases=False))
    print(f"Foram encontradas {len(all_mics)} bolsas mundiais prontas para exportação!\n")

    os.makedirs("src/data", exist_ok=True)

    # 🧠 JANELA TEMPORAL SEGURA UNIVERSAL: 20 anos de dados aceites por todas as bolsas
    START_STR = "2007-01-01"
    END_STR = "2027-12-31"

    start_date_obj = pd.Timestamp(START_STR).date()

    for mic_code in all_mics:
        try:
            cal = xcals.get_calendar(mic_code)
        except Exception as e:
            print(f"⚠️ Ignorando {mic_code}: Erro ao inicializar ({e})")
            continue

        dates = pd.date_range(start=START_STR, end=END_STR, freq='D')

        # Cabeçalho dinâmico de 4 bytes (Ano u16, Mês u8, Dia u8)
        binary_data = bytearray(struct.pack("<HBB", start_date_obj.year, start_date_obj.month, start_date_obj.day))

        # Extrair os limites físicos desta bolsa específica para evitar erros OutOfBounds
        cal_start = cal.first_session.date()
        cal_end = cal.last_session.date()

        for d in dates:
            current_date = d.date()
            day_status = 0
            open_minutes = 0
            close_minutes = 0

            # 🧠 ADICIONADO: Só interroga o cal.is_session se a data estiver dentro dos limites físicos da bolsa
            if cal_start <= current_date <= cal_end and cal.is_session(current_date):
                day_status = 1
                session_ts = pd.Timestamp(current_date)

                open_utc = cal.session_open(session_ts)
                close_utc = cal.session_close(session_ts)

                if open_utc and close_utc:
                    open_local = open_utc.tz_convert(cal.tz).time()
                    open_minutes = open_local.hour * 60 + open_local.minute

                    close_local = close_utc.tz_convert(cal.tz).time()
                    close_minutes = close_local.hour * 60 + close_local.minute

                if session_ts in cal.early_closes:
                    day_status = 2

            day_bytes = struct.pack("<BHH", day_status, open_minutes, close_minutes)
            binary_data.extend(day_bytes)

        sanitized_mic = mic_code.lower().replace("/", "_")
        filename = f"src/data/{sanitized_mic}.bin"

        with open(filename, "wb") as f:
            f.write(binary_data)

        print(f"✅ {mic_code} concluído -> {filename} ({len(binary_data) / 1024:.1f} KB)")

if __name__ == "__main__":
    export_all_markets()
