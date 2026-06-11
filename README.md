Visão geral do projeto
Objetivo: bot Solana (Rust) que copia operações de carteiras alvo (buy/sell/transfer/venda parcial) em Pump.fun/PumpSwap com foco em latência, começando em “modo simulado” (sem gastar gas) e depois ativando “modo real” com rotas privadas (Jito/bloXroute/RPC premium).

Princípios:

Arquitetura em camadas, para depois trocar facilmente free tier → infra paga.

Três modos de execução: paper (só calcula), simulado (simulateTransaction/bundle), real (send).

Telemetria forte desde o V1 para saber se faz sentido gastar com infra.

Parte 1 – Setup e organização do projeto
1.1. Repositório e estrutura

Defina desde o início módulos separados:

ingestion – tudo que lê dados on‑chain / APIs externas.

classifier – entende se um evento é buy, sell, venda parcial, transferência, etc.

strategy – decide se copia ou não, e em qual tamanho/priority/tip.

executor – cuida de simular ou enviar transações/bundles.

telemetry – logs, métricas, export para Telegram/painel.

Essa separação é o que permite depois trocar o backend de infra (por exemplo, JSON‑RPC → gRPC/Yellowstone) sem mexer na lógica de negócio.

1.2. Configuração via arquivo/env

Planeje desde o início um arquivo de config (ou .env) com coisas como:

RPC_URL (Helius/QuickNode/etc.)

Modo de execução: PAPER / SIMULATED / LIVE

Chaves/URLs de Pump.fun, Bitquery, etc.

Token do bot do Telegram.

Assim, mudar de free tier para plano pago/lotroca de provider vira só trocar config.

Parte 2 – Fonte de eventos on‑chain (ingestion básico)
Antes de pensar em copiar, você precisa “ver” as transações e eventos relevantes.

2.1. RPC/WebSocket padrão (free tier)

Comece sem gRPC, usando WebSockets do RPC padrão, que são suportados pelos principais providers (Helius, QuickNode, Shyft, etc.).

Documentação base de WebSockets Solana:

Visão geral de WebSockets Solana (vários tipos de subscription – account, logs, block, slot, etc.): 
https://docs.shyft.to/solana/rpc-calls/solana-websockets

Método logsSubscribe para assinar logs de programas (útil para Pump.fun/PumpSwap e DEXes): 
https://solana.com/docs/rpc/websocket/logssubscribe

Nesta fase, seu objetivo é:

Abrir uma conexão WebSocket com o RPC (usando a URL ws do provider).

Criar subscriptions em:

logs de programas relevantes (Pump.fun, PumpSwap, DEX que você quiser);

ou em contas específicas (carteiras alvo) se fizer sentido.

Receber eventos em tempo real e logar em um arquivo ou banco (mesmo que simples) para análise.

Você ainda não copia nada. Só garante que você está vendo os eventos certos com latência aceitável.

2.2. Fonte de dados avançada de Pump.fun (opcional nesta fase)

Para enriquecer testes (MC, price, etc), você pode usar Bitquery Pump.fun API, que já dá trades, OHLCV, market cap e migrações para PumpSwap por GraphQL/streams.

Docs:

Pump.fun API (Bitquery): 
https://docs.bitquery.io/docs/blockchain/Solana/Pumpfun/Pump-Fun-API/

Use isso inicialmente offline (scripts de análise) para entender o comportamento das carteiras alvo e calibrar as regras de strategy.

Parte 3 – Parsing e classificação de eventos (classifier)
Agora que você tem eventos, precisa transformar em “intenção de trade”.

3.1. Entender o formato de logs/txs

Usando os dados que chegam via logsSubscribe e/ou transações completas, identifique:

compra (swap SOL → token em Pump.fun ou DEX);

venda (swap token → SOL);

venda parcial (volume movido < saldo total do token);

transferência (token sai de carteira alvo para outra).

Aqui é trabalho de engenharia/negócio: decidir quais padrões de logs, contas tocadas, programas usados, etc, significam cada tipo.

3.2. Marcar a “ação copiável”

Para cada evento, seu classifier deve gerar algo tipo:

Action::Buy { mint, amount_sol, price, slot, tx_origin }

Action::Sell { mint, amount_tokens, ... }

Action::PartialSell { percentage, ... }

Action::Transfer { direction, amount, ... }

Isso é o input para a strategy. Não se preocupe com execução ainda.

Parte 4 – Camada de estratégia (quando copiar e quanto)
Aqui você transforma “carteira X comprou Y” em “o que o seu bot faria”.

4.1. Regras básicas

Defina algumas regras desde cedo:

copiar só carteiras em whitelist;

limites de valor por trade (mínimo e máximo em SOL);

filtros por MC inicial, liq, supply (usando Bitquery Pump.fun, se quiser).

4.2. Modo PAPER trading

Implemente primeiro um modo PAPER, que não monta transação, só calcula:

“se eu copiasse, compraria Z tokens com W SOL”;

guarda isso com timestamp, slot, preço, etc;

recalcula PnL teórico depois usando dados de preços/MC.

Esse modo não usa simulateTransaction nem envia nada, é só lógica em memória/banco para validar a estratégia.

Parte 5 – Integração com Pump.fun / PumpSwap para montar ordens
Agora você precisa de uma forma rápida de montar transações de buy/sell.

5.1. Pump.fun Local Trading API (recomendado)

Docs oficiais:

Local Trading API: 
https://pumpportal.fun/local-trading-api/trading-api/

Essa API recebe parâmetros como:

publicKey – sua wallet;

action – "buy" ou "sell";

mint – token (o texto depois da / na URL do Pump.fun);

amount – valor em SOL ou em tokens (pode ser "100%" para vender tudo);

denominatedInSol – se o amount é em SOL ou tokens;

slippage, priorityFee, pool etc.

Em resposta, você recebe uma transação serializada, pronta para assinar e enviar no seu RPC customizado.

Isso resolve toda a parte difícil de CPIs e instruções Pump.fun, e é perfeito para seu bot.

5.2. Pump.fun Lightning API (opcional)

Outro endpoint útil:

Lightning Transaction API: 
https://pumpportal.fun/trading-api/

Esse endpoint já simula e envia para você (dependendo de config), mas como você quer controlar simulação/manual, o local API é mais alinhado.

Parte 6 – Executor com modos PAPER / SIMULATED / LIVE
Essa é a parte crítica. A ideia é que a mesma pipeline funcione em três modos.

6.1. Modo SIMULATED usando simulateTransaction

Solana tem o RPC simulateTransaction:

Oficial: 
https://solana.com/docs/rpc/http/simulatetransaction

Outros providers (exemplos):

Helius: 
https://www.helius.dev/docs/api-reference/rpc/http/simulatetransaction

Chainstack: 
https://docs.chainstack.com/reference/solana-simulatetransaction

QuickNode: 
https://www.quicknode.com/docs/solana/simulateTransaction

Características importantes:

simula uma transação (com blockhash válido) sem broadcast e te devolve logs, errors, compute, etc;

não consome gas on‑chain, só créditos de RPC.

Fluxo no modo SIMULATED:

Strategy decide copiar.

Você chama Pump.fun Local Trading API → recebe tx serializada.

Em vez de enviar, você chama simulateTransaction nessa tx.

Você loga resultado: se seria sucesso, slot alvo, preço, etc.

Nada é enviado para a rede. É exatamente o que você descreveu: “simula como se fosse real, mas dinheiro não sai”.

6.2. Modo LIVE (quando você for ativar real)

Quando estiver confiante:

SUBSTITUI simulateTransaction por sendTransaction do provider.

Ou, se estiver usando Jito/bloXroute, passa a mandar bundles/tx via APIs deles (isso você pluga depois, mas a interface do executor já tem que prever).

Importante: mantenha o mesmo log/telemetria, só adicionando a info de assinatura on‑chain.

Parte 7 – Telemetria e logs (já pensando no painel)
Aqui é onde você começa a se preparar para o “painel Telegram/iPhone”.

7.1. Quais campos logar

Para cada ação (mesmo em simulação), logue:

tx de origem (assinatura, slot);

“sua” tx (ou “sua tx simulada”);

bloco/slot e, se possível, posição aproximada no bloco (quando usar block/slot streams);

preço (tokens por SOL) no momento;

MC na origem e MC na sua execução (usando Bitquery Pump.fun/PumpSwap).

7.2. Armazenamento

Pode ser:

um banco leve (Postgres, SQLite) ou

até JSON/CSV no começo, mas idealmente algo consultável para o painel.

A chave é ter tudo desde o V1, mesmo em papel/simulação.

Parte 8 – Bot do Telegram como painel/controle
Seu painel principal vai ser um bot no Telegram. Isso é bem estável e gratuito.

8.1. Criar o bot e pegar o token

Passos oficiais:

Telegram Bot API (docs principais): 
https://core.telegram.org/bots/api

Guia de criação com BotFather (passo a passo): 
https://apidog.com/blog/beginners-guide-to-telegram-bot-api/

Resumo do fluxo segundo esse guia:

abrir o Telegram;

falar com @BotFather, usar /newbot;

BotFather te dá o token do bot, que você vai usar no backend.

8.2. O que o bot deve fazer neste projeto

Comece simples:

comandos para ver status (/status, /last_trades);

ver PnL teórico/real por carteira alvo;

alternar config simples (/set_mode paper|simulated|live, /set_priority_fee, etc.).

Você pode usar qualquer SDK (Rust, Node, Python). Se optar por uma camada fora do core (por exemplo, um microserviço em Node/Python só para Telegram), libs comuns são:

Node: 
https://www.npmjs.com/package/node-telegram-bot-api

Python: 
https://docs.python-telegram-bot.org

O importante é: o core do bot de cópia continua em Rust; o serviço do Telegram só consome API/DB do seu core.

Parte 9 – Roadmap de testes (o que validar em cada etapa)
Pensa em três níveis de teste:

Paper

ingestion + classifier + strategy funcionando;

nenhum uso de simulateTransaction;

checar se as decisões fazem sentido com dados históricos e stream atual.

Simulation

mesma pipeline, mas usando simulateTransaction com txs Pump.fun.

logs mostram “sim teria sido fill / sim teria falhado / sim teria revertido”.

calibrar slippage, priorityFee, amount.

Live controlado

habilita LIVE, mas com limite diário de SOL (tipo 0.1 SOL/dia) e valor máximo por trade;

monitora taxas, sucesso de inclusão, PnL real.

Você só passa de um nível para o outro quando o anterior estiver estável (sem crash, sem bugs óbvios).

Parte 10 – Preparando o caminho para infra avançada
Como você já estruturou em camadas, o upgrade para coisas pagas fica natural:

Trocar ChainClient para um GrpcYellowstoneClient usando os devidos endpoints (Helius/QuickNode/Chainstack) quando/quiser pagar por gRPC/stream dedicado.

Trocar ExecutionBackend para usar Jito bundles + simulateBundle, seguindo docs como:

Jito low‑latency / bundles: 
https://docs.jito.wtf/lowlatencytxnsend/

(E, dependendo do provider, métodos como simulateBundle: 
https://www.quicknode.com/docs/solana/simulateBundle
)

Integrar bloXroute Trader API de forma opcional para rota privada adicional, usando o plano Intro/free no início.

Nada disso exige reescrever o core, desde que você siga o plano de interfaces para chain/executor.