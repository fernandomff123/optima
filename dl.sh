#!/bin/bash

# 🌟 1. CONFIGURAÇÃO DA LISTA DE TICKERS: Adicione ou remova os ativos aqui separados por espaço
LISTA_TICKERS=("SPY" )

# 🌟 2. CAPTURA A DATA: Se passar argumento (ex: ./baixar_cadeias.sh 09_06_2026), usa-o.
# Se omitir, calcula dinamicamente a data de hoje no formato DD_MM_YYYY.
DATA_ALVO=${1:-$(date +"%d_%m_%Y")}

echo "===================================================================="
echo "🚀 INICIANDO DOWNLOAD EM LOTE DO INSTANTE DA CBOE"
echo "📅 Data de Referência do Nome: ${DATA_ALVO}"
echo "===================================================================="

# Loop 'for' para percorrer cada empresa da lista de forma automática
for TICKER in "${LISTA_TICKERS[@]}"
do
    # Nome do ficheiro de saída dinâmico com o Ticker e a Data
    NOME_ARQUIVO="${TICKER}_CHAIN_${DATA_ALVO}.json"

    echo "🌐 Descarregando opções de: ${TICKER}..."
    # Executa o curl injetando a variável do Ticker no link da CBOE

    curl -H "User-Agent: Mozilla/5.0" "https://cdn.cboe.com/api/global/delayed_quotes/options/${TICKER}.json" -o  "options.json"
    echo "💾 Salvo com sucesso como: ${NOME_ARQUIVO}"
    echo "--------------------------------------------------------------------"
done

echo "✅ OPERAÇÃO CONCLUÍDA! Todos os ficheiros foram guardados e rotulados."







