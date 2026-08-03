
#!/bin/bash

# Configurações do teu pipeline
FICHEIRO_FINAL="options.json"
FICHEIRO_TMP="options.json.tmp"
INTERVALO_SEGUNDOS=60
TICKER="SPY"

echo "🚀 Ingestor Atómico Iniciado. A atualizar a cada ${INTERVALO_SEGUNDOS}s para o ativo ${TICKER}..."

while true; do
    AGORA=$(date +"%Y-%m-%d %H:%M:%S")
    echo "📥 [${AGORA}] A descarregar nova cadeia de opções do mercado aberto da Cboe..."

    # 1. FAZER O DOWNLOAD DIRECTO PARA O FICHEIRO TEMPORÁRIO (.tmp)
    # Removido o curl duplo de teste. Mantido apenas o feed autêntico da Cboe com o User-Agent
    curl -s -H "User-Agent: Mozilla/5.0" "https://cdn.cboe.com/api/global/delayed_quotes/options/${TICKER}.json" -o "$FICHEIRO_TMP"

    # Capturar o status do download imediatamente após o comando curl
    STATUS_DOWNLOAD=$?

    # Validação rigorosa: só avança se o status for 0 E o ficheiro temporário contiver dados reais
    if [ $STATUS_DOWNLOAD -eq 0 ] && [ -s "$FICHEIRO_TMP" ]; then

        # 2. SUBSTITUIÇÃO ATÓMICA DO KERNEL
        # O ficheiro passa de antigo a novo instantaneamente. O monitor do Rust nunca falha!
        mv "$FICHEIRO_TMP" "$FICHEIRO_FINAL"

        HORA_AGORA=$(date +"%H:%M:%S") # <-- CORRIGIDO: Comando isolado numa variável limpa
        echo "✅ [${HORA_AGORA}] Ficheiro '${FICHEIRO_FINAL}' atualizado com sucesso de forma atómica."
    else
        echo "❌ [ERROR] Falha no download do feed da Cboe ou dados vazios. A saltar tick..."
        rm -f "$FICHEIRO_TMP"
    fi

    # 3. Temporizador exato de 1 minuto
    echo "⏳ A aguardar ${INTERVALO_SEGUNDOS} segundos para o próximo tick..."
    sleep $INTERVALO_SEGUNDOS
    echo "--------------------------------------------------------------------------"
done
