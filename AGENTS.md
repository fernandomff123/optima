# AGENTS.md — Optima

Este ficheiro contém regras permanentes para qualquer agente Codex que trabalhe no Optima.

Ler integralmente este ficheiro antes de analisar, alterar ou criar código.

## 1. Princípios gerais

* Preservar a arquitetura hexagonal no backend e no frontend.
* Respeitar as fronteiras e convenções já existentes no repositório.
* Preferir alterações pequenas, explícitas, testáveis e fáceis de rever.
* Não introduzir dependências ou abstrações sem uma necessidade concreta.
* Não misturar refactors, alterações funcionais e mudanças visuais no mesmo trabalho.
* Não alterar contratos públicos, persistência, schemas ou semântica financeira sem autorização explícita.
* Não inventar dados, métricas ou regras financeiras.
* Não fazer commit, push, merge, rebase ou force push sem pedido explícito.
* Nunca apagar ou substituir trabalho existente sem primeiro inspecionar o estado Git.
* Preservar alterações do utilizador que não pertençam à tarefa atual.
* Não consultar `prototipo/`, stashes ou bases de dados de produção salvo pedido explícito.

## 2. Arquitetura hexagonal obrigatória

O backend e o frontend seguem a mesma filosofia:

```text
driving adapter
    → driving port
        → application
            → domain
            → driven port
                → driven adapter
```

A direção das dependências aponta sempre para o interior.

### 2.1 Domínio

O domínio:

* contém conceitos, invariantes, métricas e regras de negócio;
* é independente de frameworks, transporte, persistência, providers e interface;
* não conhece Axum, Leptos, Plotly, HTTP, JSON, DuckDB, SQLite, Yahoo, CBOE ou Treasury;
* não depende de adapters concretos;
* não contém DTOs específicos de HTTP ou de providers;
* não usa tempo implícito quando o instante de referência influencia o resultado;
* rejeita estados numericamente inválidos quando estes não representam valores financeiros válidos.

Cálculos financeiros, convenções, elegibilidade de contratos, interpolação e validação numérica não podem ser executados em handlers, componentes visuais ou mappers de transporte.

### 2.2 Application

A application:

* implementa casos de uso;
* coordena domínio e ports;
* define projeções e read models provider-neutral exigidos pelo caso de uso;
* não conhece frameworks nem adapters concretos;
* não devolve tipos de Axum, Leptos, Plotly, DuckDB ou providers externos;
* não contém detalhes de renderização ou transporte;
* recebe explicitamente contexto como ativo, intervalo, data ou instante quando necessário.

### 2.3 Ports

Os ports:

* representam conversas estáveis entre o núcleo e o exterior;
* usam tipos do domínio ou modelos provider-neutral;
* não expõem tipos de frameworks, bases de dados ou providers;
* devem ser evoluídos quando uma conversa existente já representa o novo caso de uso;
* não devem ser duplicados apenas para contornar uma fronteira arquitetural.

### 2.4 Adapters

Os adapters:

* implementam detalhes de UI, HTTP, persistência, providers, relógio ou visualização;
* convertem DTOs externos para modelos internos na fronteira;
* não decidem regras financeiras;
* não duplicam políticas existentes no domínio ou application;
* não transportam tipos concretos para o interior do hexágono.

## 3. Backend

Manter a organização existente:

```text
src/
├── hexagon/
│   ├── domain/
│   ├── application/
│   ├── driving_ports/
│   └── driven_ports/
├── driving_adapters/
└── driven_adapters/
```

Regras adicionais:

* handlers HTTP limitam-se a validar transporte, chamar um driving port e mapear a resposta;
* mappers HTTP não executam cálculos financeiros;
* adapters de persistência não definem políticas de negócio;
* adapters de providers não propagam modelos específicos do provider para o domínio;
* fontes externas só podem ser chamadas através dos driven adapters do backend;
* evitar `unwrap()`, `expect()` e `panic!()` em código de produção;
* erros devem preservar contexto suficiente e atravessar fronteiras de forma explícita;
* manter unidades financeiras documentadas e inequívocas;
* não abrir ou modificar a DuckDB normal durante testes ou auditorias, salvo autorização explícita.

## 4. Frontend

O frontend é uma aplicação Leptos CSR e deve seguir arquitetura hexagonal.

Tecnologias aprovadas:

* Leptos em modo CSR;
* Tailwind CSS;
* Plotly;
* Rust;
* mocks determinísticos durante a fase de construção visual.

O frontend comunica exclusivamente com o backend Optima.

É proibido o browser comunicar diretamente com Yahoo, CBOE, Treasury ou qualquer outra fonte externa de dados.

Os pedidos reais devem ser same-origin e entrar pelas rotas do backend, normalmente sob `/api`.

### 4.1 Organização aprovada

A organização aprovada combina camadas hexagonais globais, organizadas internamente por feature:

```text
web/src/
├── main.rs
├── composition.rs
├── domain/
│   ├── asset/
│   ├── navigation/
│   ├── filters/
│   └── simulation/
├── application/
│   ├── read_models/
│   ├── asset_workspace/
│   ├── chart/
│   ├── options/
│   ├── volatility/
│   ├── gex/
│   ├── simulation/
│   └── portfolio/
├── ports/
├── driven_adapters/
│   ├── mocks/
│   └── http/
├── driving_adapters/
│   └── ui/
│       ├── router/
│       ├── layouts/
│       ├── pages/
│       ├── components/
│       └── plotly/
└── design_system/
```

A estrutura pode ser refinada durante a implementação, mas as fronteiras não podem ser removidas.

### 4.2 Domínio do frontend

O domínio do frontend é fino e contém apenas conceitos próprios da experiência do utilizador:

* contexto do ativo;
* identidade apresentada;
* capacidades disponíveis;
* tabs e navegação;
* filtros válidos;
* seleção;
* drafts de simulação ainda não enviados;
* preferências estritamente locais quando autorizadas.

O domínio do frontend não contém um segundo domínio financeiro. Não implementar novamente no frontend:

* pricing;
* Greeks;
* Options Chain financeira;
* GEX;
* volatilidade implícita;
* interpolação;
* Monte Carlo;
* VaR ou CVaR;
* convenções financeiras;
* regras canónicas já pertencentes ao backend.

O domínio do frontend não pode importar ou conhecer:

* `leptos`;
* Plotly;
* `gloo`;
* `web_sys`;
* HTTP;
* DTOs da API;
* CSS ou Tailwind;
* tipos específicos de componentes.

`presentation/` não pertence dentro de `domain/`. `DataState`, freshness, métricas formatadas, modelos de tabela e chart models são read models da application ou modelos de apresentação. Não chamar “domínio” a estruturas criadas apenas para renderização. Chart models permanecem provider-neutral e não conhecem Plotly.

### 4.3 Application do frontend

A application do frontend:

* implementa os casos de uso de cada ecrã;
* solicita dados através de ports;
* coordena filtros, seleção do ativo e intervalo;
* produz modelos de apresentação provider-neutral;
* representa explicitamente estados de carregamento, sucesso, ausência, stale e erro;
* não cria componentes Leptos;
* não cria traces ou layouts Plotly;
* não faz pedidos HTTP diretamente.

### 4.4 Ports e abstrações

* Criar um port apenas quando existir uma dependência externa concreta exigida por um caso de uso.
* Não criar antecipadamente ports para watchlist, realtime, catálogo, storage ou preferências.
* Mocks e HTTP implementam o mesmo port quando o adapter HTTP existir.
* Não criar um `ChartRendererPort`.
* Plotly é um adapter concreto de visualização.
* Não criar repositories, services, utils ou builders genéricos sem necessidade demonstrada.

### 4.5 UI Leptos

Os componentes Leptos são driving adapters.

Os componentes:

* recebem estado e emitem intenções do utilizador;
* chamam casos de uso da application;
* não chamam `gloo-net` ou outro cliente HTTP diretamente;
* não executam cálculos financeiros;
* não conhecem DTOs do backend;
* não contêm grandes blocos de transformação de dados;
* não decidem unidades, convenções ou elegibilidade financeira;
* devem ser pequenos, reutilizáveis e acessíveis;
* devem representar loading, unavailable, stale e error de forma explícita;
* não devem montar todos os gráficos de todas as rotas simultaneamente.

As tabs visuais que representam módulos diferentes devem corresponder a rotas reais e partilháveis. O routing é real e os gráficos são carregados apenas na respetiva rota, com lifecycle Plotly explícito.

### 4.6 Estratégia mocks-first, HTTP e DTOs

A sequência aprovada é:

```text
fundação
    → ecrãs com mocks determinísticos
        → validação visual e funcional
            → contratos backend em falta
                → integração HTTP página a página
```

Durante a primeira fase, os componentes usam casos de uso e ports e os mocks ficam centralizados. O adapter HTTP e o adapter de mocks implementam o mesmo port quando o HTTP existir.

```text
Componente Leptos
    → caso de uso
        → port
            → mock ou HTTP
```

Regras:

* trocar mocks pelo backend não pode obrigar a reescrever os componentes;
* mocks são centralizados, determinísticos e claramente identificados;
* não espalhar valores mock diretamente pelos componentes;
* não ligar prematuramente cada página ao HTTP;
* mocks podem simular estados disponíveis, stale, indisponíveis e erros;
* métricas ausentes no backend podem aparecer apenas como mock claramente identificado durante validação visual e nunca como funcionalidades reais;
* DTOs HTTP são convertidos na fronteira;
* os componentes nunca recebem diretamente DTOs HTTP;
* não introduzir chamadas externas no adapter HTTP do frontend;
* não criar cálculos financeiros falsos apenas para preencher os desenhos.

### 4.7 Plotly

Plotly é um adapter de visualização.

* Manter configuração Plotly fora dos componentes de página.
* Criar mappers pequenos por família de gráfico apenas quando houver utilização concreta.
* Receber modelos provider-neutral da application.
* Não executar políticas financeiras dentro dos builders Plotly.
* Preservar unidades, ordenação, escalas e significado dos dados.
* Evitar configuração Plotly duplicada.
* Carregar gráficos apenas nas rotas onde são necessários.
* Testar separadamente transformações relevantes entre read models e traces.
* Reutilizar os tokens visuais aprovados e não usar cores default do Plotly quando divergirem da baseline.

### 4.8 Tailwind, design system e componentes

* Usar Tailwind como sistema principal de styling.
* Evitar CSS global extenso e estilos ad hoc repetidos.
* Centralizar cores, tipografia, espaçamento, estados e tokens visuais.
* Criar primitives e componentes reutilizáveis apenas quando existirem utilizações concretas.
* Preservar consistência entre sidebar, headers, tabs, cards, tabelas, filtros e estados.
* Não codificar arbitrariamente valores visuais repetidos em vários componentes.
* Manter comportamento responsivo e acessibilidade por teclado.
* Não sacrificar legibilidade financeira para reproduzir decoração das referências.

A reutilização visual deve acontecer, por esta ordem, através de:

1. tokens Tailwind;
2. componentes Leptos concretos;
3. utility classes;
4. `@layer components` apenas para padrões CSS pequenos e realmente repetidos.

Não recriar CSS tradicional através de uma grande coleção de classes com `@apply`.

Componentes comuns aprovados como direção, apenas quando usados concretamente:

* `AppShell`;
* `Sidebar`;
* `GlobalSearch`;
* `AssetHeader`;
* `AssetTabs`;
* `PageToolbar`;
* `Panel`;
* `MetricStrip`;
* `ChartFrame`;
* `DataTable`;
* `OptionsChain`;
* `FilterBar`;
* `FreshnessBadge`;
* estados loading, stale, unavailable, empty e error.

### 4.9 Baseline visual e cores

A família de imagens com nomes semânticos em `Bloomberg/v2/` é a baseline visual. As outras imagens podem contribuir com ecrãs ou subvistas ausentes, mas não podem substituir o shell ou a paleta baseline.

As cores observadas e aprovadas, extraídas dessas imagens, são:

```text
canvas             #030C17
sidebar            #040E1A
surface            #08111A
surface-elevated   #0E1A27
border             #1C2530
chart-grid         #212935
text-primary       #FFFFFF
text-secondary     #A6AAAF
text-muted-source  #4F5862
interactive-source #1B5DCA
finance-positive   #35A95C
finance-negative   #DE3436
level-special      #E38113
state-hover        #0A1521
state-selected     #173A6F
state-focus        #1D62D2
```

Estas cores não podem ser substituídas por uma paleta arbitrária. Quando a mesma função variar noutra família visual, prevalece a cor da baseline, sem calcular médias.

Para acessibilidade:

* preservar as cores originais em backgrounds, gráficos, linhas, fills e elementos decorativos;
* não alterar silenciosamente a identidade visual;
* quando uma cor original falhar contraste como texto, criar um token textual adicional;
* não substituir globalmente o token original.

Variantes textuais aprovadas para os contextos que exigem contraste adicional:

```text
text-muted-readable #78828D
interactive-text    #3B82F6
negative-text       #EE4547
```

O tema Plotly deve reutilizar exatamente os mesmos tokens para paper background, plot background, grelha, eixos, labels, traces positivas e negativas, seleção, hover e níveis especiais.

### 4.10 Decisões do MVP

* O frontend é desktop escuro e denso, baseado na família semanticamente nomeada.
* O ativo é o contexto central.
* Cada módulo usa uma rota real.
* Símbolo/ticker é o identificador inicial de rota enquanto o backend não fornecer um ID estável.
* As tabs iniciais são Overview, Chart, Options, Volatility, GEX e Simulation.
* O Chart inicial contém OHLC/candlestick e volume, sem RSI, MACD ou drawing tools no MVP.
* Options Chain é uma tabela HTML, não um gráfico Plotly.
* Em mobile, Options Chain usa uma apresentação adaptada sem esmagar todas as colunas.
* Portfolio entra depois dos módulos principais do ativo.
* News, Alerts e Events ficam fora enquanto não existirem contratos.
* Monte Carlo, POP, VaR e CVaR ficam fora enquanto não existirem domínio e contratos.
* Não mostrar Call Wall ou Put Wall sem definição financeira no backend.
* Não mostrar IV Rank, IV Percentile ou 25-delta skew sem contrato.
* `nearest_zero_crossing` não pode ser rotulado “Gamma Flip” sem decisão financeira explícita.
* GEX apresenta exatamente unidades e convenções fornecidas pelo backend.
* Não inferir regime long/short gamma no frontend.
* Realtime não pode ser simulado como funcionalidade real sem contrato.

### 4.11 Rotas de referência

A direção atual é:

```text
/
/markets
/assets
/assets/:ticker/overview
/assets/:ticker/chart
/assets/:ticker/options
/assets/:ticker/volatility
/assets/:ticker/gex
/assets/:ticker/simulation
/options
/volatility
/gex
/simulations
/portfolio
/settings
```

Rotas sem contratos, como News, não entram inicialmente. Subvistas analíticas podem usar query parameters, desde que sejam partilháveis e validadas:

```text
/assets/:ticker/volatility?view=...
/assets/:ticker/gex?view=...
/assets/:ticker/simulation?view=...
```

## 5. Referências Bloomberg

Apenas `Bloomberg/v2/` é referência ativa. `Bloomberg/v1/` é arquivo histórico e não orienta a implementação.

Os PDFs são material editorial ou financeiro, não especificação de UI. As imagens são referências de produto e composição desktop, não uma especificação pixel-perfect. Os valores apresentados são ilustrativos e não podem ser transformados em regras financeiras.

Antes de construir o frontend:

* inventariar todos os ficheiros;
* identificar ecrãs, variações e estados;
* relacionar cada desenho com uma rota;
* identificar elementos partilhados;
* identificar dados meramente ilustrativos;
* distinguir informação que o backend já fornece da que ainda não existe;
* ler os textos de apoio antes de propor a implementação;
* apresentar o inventário e as ambiguidades antes de começar alterações extensas.

Loading, stale, unavailable, empty e error devem ser desenhados mesmo quando não aparecem nas imagens. Acessibilidade e responsividade são obrigatórias; a densidade desktop não deve ser simplesmente comprimida em ecrãs pequenos.

As referências não autorizam a invenção de métricas ou contratos financeiros.

Ao adicionar `Bloomberg/` ao Git:

* verificar primeiro nomes, extensões, tamanhos e quantidade de ficheiros;
* preservar os originais;
* não redimensionar, recomprimir, renomear ou eliminar imagens sem autorização;
* não incluir ficheiros temporários, caches ou metadados irrelevantes;
* confirmar que nenhum ficheiro contém credenciais, tokens ou informação sensível;
* verificar limites de tamanho do GitHub antes do commit;
* se algum ficheiro exceder os limites normais, parar e apresentar opções;
* mostrar exatamente o que será incluído antes de fazer commit;
* só fazer commit e push mediante pedido explícito.

Usar sempre a grafia e capitalização reais do diretório: `Bloomberg/`.

## 6. Tamanho e responsabilidade dos ficheiros

Evitar ficheiros grandes e componentes monolíticos.

Orientações:

* um ficheiro deve ter uma responsabilidade principal;
* preferir ficheiros abaixo de aproximadamente 250–300 linhas;
* antes de ultrapassar 350 linhas, avaliar divisão por responsabilidade;
* não usar o limite para criar fragmentação artificial;
* testes parametrizados, fixtures e código gerado podem justificar exceções;
* handlers, componentes, casos de uso e mappers extensos devem ser divididos;
* `app.rs`, `main.rs`, `mod.rs` e ficheiros de rotas não devem concentrar a aplicação inteira;
* não criar módulos genéricos como `utils.rs` para acumular responsabilidades não relacionadas.

## 7. Testes arquiteturais

Adicionar e manter testes que impeçam regressões das fronteiras.

No mínimo, proteger que:

* domínio não depende de adapters ou frameworks;
* application não depende de adapters concretos;
* HTTP não contém cálculos financeiros;
* componentes Leptos não chamam HTTP diretamente;
* componentes não importam DTOs HTTP;
* Plotly não entra no domínio ou application;
* mocks e HTTP implementam os mesmos ports;
* o frontend não referencia fontes externas de dados;
* regras financeiras não são duplicadas em mappers ou UI.

Quando uma violação arquitetural for corrigida, adicionar um teste que impeça o seu regresso sempre que isso for razoável.

## 8. Processo de trabalho

Antes de editar:

1. Ler este `AGENTS.md`.
2. Confirmar branch e `git status`.
3. Inspecionar alterações existentes.
4. Identificar as fronteiras arquiteturais envolvidas.
5. Procurar implementações e consumidores existentes.
6. Distinguir regras financeiras, coordenação, transporte e apresentação.
7. Parar e perguntar se existir uma ambiguidade que altere comportamento ou contrato.

Durante a implementação:

* trabalhar apenas no âmbito pedido;
* não corrigir assuntos adjacentes sem autorização;
* preservar contratos existentes salvo decisão explícita;
* reutilizar conceitos existentes antes de criar novos;
* não deslocar uma violação de arquitetura para outro mapper ou helper;
* não duplicar lógica para evitar alterar um port adequado;
* manter o diff pequeno e legível;
* atualizar testes juntamente com o código.

No final:

* explicar fluxo anterior e final;
* listar ficheiros alterados;
* indicar testes adicionados ou atualizados;
* apresentar validações executadas;
* apresentar `git diff --stat`;
* apresentar `git status --short`;
* documentar limitações e decisões deliberadas;
* não afirmar que algo passou se não tiver sido realmente executado.

## 9. Validações obrigatórias

Executar, conforme aplicável:

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets
git diff --check
```

Para o frontend, executar também as validações configuradas no repositório para:

* build Leptos CSR;
* Tailwind;
* Trunk;
* testes do frontend;
* testes arquiteturais.

Em tarefas limitadas ao frontend, executar validações apenas para o package `optima-web` e respetivo build CSR. Não executar check, clippy ou test do workspace completo quando backend, crates partilhadas e contratos não foram alterados. Validações backend ou workspace-wide só são exigidas quando o âmbito da alteração possa afetá-los.

Se alguma validação não estiver ainda configurada, declarar isso explicitamente. Não fingir que foi executada.

Não abrir a DuckDB normal apenas para validar uma alteração. Usar fixtures, mocks ou bases de dados temporárias de teste.

## 10. Git e segurança

* Inspecionar sempre `git status` antes de alterar ficheiros.
* Não apagar ficheiros untracked sem confirmar a sua origem.
* Não usar `git reset --hard`.
* Não usar `git checkout --` para descartar alterações do utilizador.
* Não fazer force push.
* Não incluir `prototipo/` num commit.
* Não consultar stashes sem autorização.
* Não incluir bases de dados, segredos, ficheiros `.env` ou credenciais.
* Um commit deve representar uma alteração coerente.
* Um PR deve ter âmbito limitado e documentar arquitetura, comportamento e testes.
* Criar Draft PR por omissão quando for pedida publicação, salvo indicação contrária.
* Não marcar Ready nem fazer merge sem autorização explícita.

## 11. Prioridade das decisões

Em caso de conflito:

1. preservar integridade financeira e segurança;
2. preservar contratos públicos válidos;
3. respeitar arquitetura hexagonal;
4. respeitar instruções explícitas da tarefa;
5. manter consistência com código adjacente;
6. minimizar complexidade e dimensão do diff.

Se a evolução correta não for inequívoca, parar antes de editar e apresentar a dúvida de forma concreta.
