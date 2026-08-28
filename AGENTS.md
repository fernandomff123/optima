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

### 4.1 Organização pretendida

Usar esta estrutura como orientação:

```text
web/src/
├── domain/
│   ├── assets/
│   ├── metrics/
│   ├── charts/
│   ├── options/
│   └── portfolio/
├── application/
│   ├── asset_explorer/
│   ├── market_overview/
│   ├── volatility/
│   ├── options/
│   ├── gex/
│   └── simulation/
├── ports/
├── driven_adapters/
│   ├── http/
│   └── mocks/
├── driving_adapters/
│   └── ui/
│       ├── routes/
│       ├── layouts/
│       ├── pages/
│       └── components/
├── charting/
│   └── plotly/
├── design_system/
└── main.rs
```

A estrutura pode evoluir, mas as fronteiras arquiteturais não podem ser removidas.

### 4.2 Domínio do frontend

O domínio do frontend pode definir:

* identificadores e categorias de ativos;
* métricas e respetivas unidades;
* séries, superfícies e perfis financeiros;
* semântica dos gráficos;
* tipos abstratos de visualização;
* intervalos, filtros e seleções válidas;
* estados e invariantes relevantes para o utilizador.

O domínio do frontend não pode importar ou conhecer:

* `leptos`;
* Plotly;
* `gloo`;
* `web_sys`;
* HTTP;
* DTOs da API;
* CSS ou Tailwind;
* tipos específicos de componentes.

Pode existir uma intenção abstrata como série temporal, superfície, distribuição ou perfil por strike. A transformação dessa intenção em traces e layouts Plotly pertence ao adapter de visualização.

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

### 4.4 UI Leptos

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

As tabs visuais que representam módulos diferentes devem corresponder a rotas reais e partilháveis.

### 4.5 HTTP e mocks

O adapter HTTP e o adapter de mocks devem implementar os mesmos ports.

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
* mocks podem simular estados disponíveis, stale, indisponíveis e erros;
* DTOs HTTP são convertidos na fronteira;
* os componentes nunca recebem diretamente DTOs HTTP;
* não introduzir chamadas externas no adapter HTTP do frontend;
* não criar cálculos financeiros falsos apenas para preencher os desenhos.

### 4.6 Plotly

Plotly é um adapter de visualização.

* Manter configuração Plotly fora dos componentes de página.
* Criar builders ou mappers pequenos por família de gráfico.
* Receber modelos provider-neutral da application.
* Não executar políticas financeiras dentro dos builders Plotly.
* Preservar unidades, ordenação, escalas e significado dos dados.
* Evitar configuração Plotly duplicada.
* Carregar gráficos apenas nas rotas onde são necessários.
* Testar separadamente transformações relevantes entre read models e traces.

### 4.7 Tailwind e design system

* Usar Tailwind como sistema principal de styling.
* Evitar CSS global extenso e estilos ad hoc repetidos.
* Centralizar cores, tipografia, espaçamento, estados e tokens visuais.
* Criar primitives reutilizáveis apenas quando existirem utilizações concretas.
* Preservar consistência entre sidebar, headers, tabs, cards, tabelas, filtros e estados.
* Não codificar arbitrariamente valores visuais repetidos em vários componentes.
* Manter comportamento responsivo e acessibilidade por teclado.
* Não sacrificar legibilidade financeira para reproduzir decoração das referências.

## 5. Referências Bloomberg

As imagens e textos em `Bloomberg/`, especialmente `Bloomberg/v2/`, são referências de produto e de composição visual.

Antes de construir o frontend:

* inventariar todos os ficheiros;
* identificar ecrãs, variações e estados;
* relacionar cada desenho com uma rota;
* identificar elementos partilhados;
* identificar dados meramente ilustrativos;
* distinguir informação que o backend já fornece da que ainda não existe;
* ler os textos de apoio antes de propor a implementação;
* apresentar o inventário e as ambiguidades antes de começar alterações extensas.

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
