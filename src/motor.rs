//! Motor do aplicativo.
//!
//! Toda a lógica reativa (atalho global, detecção de seleção, cliques do menu
//! flutuante e da bandeja) roda nesta thread, e **não** dentro do `update()` do
//! egui. Motivo: quando a janela está escondida na bandeja o egui pode parar de
//! ser chamado, e o app precisa continuar funcionando nesse estado.

use crate::config::Config;
use crate::correcao::Aviso;
use crate::deteccao;
use crate::popup::{
    Icone, ItemBarra, PopupEvento, Popups, ACAO_NENHUMA, ACAO_ABRIR_APP, ACAO_COPIAR, ACAO_CORRIGIR,
    ACAO_CORRIGIR_COPIAR, ACAO_DICIONARIO, ACAO_FECHAR, ACAO_HISTORICO, ACAO_MAIS, ACAO_MELHORAR, ACAO_MELHORAR_CURTO,
    ACAO_MELHORAR_FORMAL, ACAO_MELHORAR_SIMPLES, ACAO_PAUSAR, ACAO_REVISAR, ACAO_SAIR,
    ACAO_SUGESTAO_BASE, MAX_SUGESTOES,
};
use crate::tray::AcaoBandeja;
use std::collections::HashSet;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone)]
pub enum Mensagem {
    /// Atalho global pressionado.
    Hotkey,
    /// Texto selecionado detectado com o mouse.
    Selecao { x: i32, y: i32, editavel: bool },
    /// Teste direto da barra flutuante (preview visual).
    TestarBarra { x: i32, y: i32 },
    /// Pedido para analisar de novo o texto selecionado (botão da janela).
    Reanalisar,
    /// Eventos das janelas de popup (escolha de ação, fechamento).
    Popup(PopupEvento),
    /// Cliques no ícone/menu da bandeja.
    Bandeja(AcaoBandeja),
}

/// Estado compartilhado entre a GUI, o motor e os threads de ação.
pub struct Compartilhado {
    pub config: Config,
    pub logs: Vec<String>,
    /// Último texto devolvido pela correção/IA (mostrado na janela e no balão).
    pub resultado: Option<String>,
    /// Estado da conexão de teste com a IA (mensagem).
    pub status_ia: String,
    /// Pedido para a GUI mostrar/esconder a janela: Some(true) = mostrar.
    pub janela: Option<bool>,
    /// Ação em andamento (evita disparar duas ao mesmo tempo).
    pub ocupado: bool,
    /// Espelho da última configuração salva pela GUI (evita salvar a cada frame).
    pub deteccao_anterior: bool,
    /// Últimos avisos da verificação (a lista que a janela mostra e a seta do
    /// "Corrigir" usa para oferecer as alternativas reais).
    pub analise: Vec<Aviso>,
    /// Texto a que `analise` se refere.
    pub analise_texto: Option<String>,
    /// Há uma verificação em andamento.
    pub analisando: bool,
    /// Contador para descartar análise antiga quando o usuário já selecionou
    /// outro texto.
    pub analise_geracao: u64,
    /// Histórico persistido de até 10 textos corrigidos mais recentes.
    pub historico: crate::historico::Historico,
}

impl Compartilhado {
    pub fn novo(config: Config) -> Self {
        let mut logs = Vec::new();
        logs.push(format!(
            "[Sistema] Corretor iniciado. Selecione um texto (menu automático) ou use o atalho. Config: {}",
            Config::caminho().display()
        ));
        let deteccao_anterior = config.deteccao_automatica;
        Self {
            config,
            logs,
            resultado: None,
            status_ia: String::new(),
            janela: None,
            ocupado: false,
            deteccao_anterior,
            analise: Vec::new(),
            analise_texto: None,
            analisando: false,
            analise_geracao: 0,
            historico: crate::historico::Historico::carregar(),
        }
    }

    pub fn log(&mut self, mensagem: impl Into<String>) {
        self.logs.push(mensagem.into());
        if self.logs.len() > 400 {
            self.logs.drain(0..100);
        }
    }
}

pub type Estado = Arc<Mutex<Compartilhado>>;

pub fn logar(estado: &Estado, mensagem: impl Into<String>) {
    if let Ok(mut compartilhado) = estado.lock() {
        compartilhado.log(mensagem);
    }
}

/// Cria o canal do motor e sobe a thread.
pub fn iniciar(estado: Estado) -> (Sender<Mensagem>, Arc<Popups>) {
    let (tx, rx) = channel::<Mensagem>();
    let popups = Arc::new(Popups::novo(tx.clone()));
    let popups_motor = Arc::clone(&popups);
    // detecção automática de seleção (thread própria)
    deteccao::iniciar(tx.clone());
    let estado_motor = Arc::clone(&estado);
    thread::spawn(move || laco(estado_motor, rx, popups_motor));
    (tx, popups)
}

fn laco(estado: Estado, rx: Receiver<Mensagem>, popups: Arc<Popups>) {
    let mut contexto = Contexto::default();
    loop {
        // mantém a detecção automática sincronizada com a configuração
        let (ativo, deteccao) = match estado.lock() {
            Ok(c) => (c.config.ativo, c.config.deteccao_automatica),
            Err(_) => (false, false),
        };
        deteccao::ligar(ativo && deteccao);

        match rx.recv_timeout(Duration::from_millis(200)) {
            Ok(mensagem) => tratar(mensagem, &estado, &popups, &mut contexto),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
}

#[derive(Default)]
struct Contexto {
    foco_editavel: bool,
    posicao: (i32, i32),
}

fn tratar(mensagem: Mensagem, estado: &Estado, popups: &Arc<Popups>, contexto: &mut Contexto) {
    match mensagem {
        Mensagem::Hotkey => {
            let ativo = estado.lock().map(|c| c.config.ativo).unwrap_or(false);
            if !ativo {
                return;
            }
            let foco = deteccao::analisar_foco();
            let editavel = foco.as_ref().map(|f| f.editavel).unwrap_or(true);
            contexto.foco_editavel = editavel;
            contexto.posicao = posicao_cursor();
            let (x, y) = contexto.posicao;
            if Popups::suportado() {
                mostrar_barra(estado, popups, x, y, true);
            } else {
                // Sem popups nativos: o atalho corrige direto (resultado na janela).
                executar(estado, popups, ACAO_CORRIGIR, editavel, x, y);
            }
        }
        Mensagem::Reanalisar => {
            // Pedido da janela: captura a seleção (UI Automation e, se precisar,
            // Ctrl+C) e verifica de novo, mostrando a lista na janela.
            verificar_selecao(estado, popups, true);
        }
        Mensagem::Selecao { x, y, editavel } => {
            let (ativo, auto) = match estado.lock() {
                Ok(c) => (c.config.ativo, c.config.deteccao_automatica),
                Err(_) => (false, false),
            };
            if !(ativo && auto) {
                return;
            }
            // nosso próprio popup/janela não deve abrir o menu de novo
            if foco_e_nosso() {
                return;
            }
            if let Some(foco) = deteccao::analisar_foco() {
                if foco.tem_text_pattern && !foco.tem_selecao {
                    return;
                }
            }
            contexto.foco_editavel = editavel;
            contexto.posicao = (x, y);
            mostrar_barra(estado, popups, x, y, true);
        }
        Mensagem::TestarBarra { x, y } => {
            contexto.foco_editavel = false;
            contexto.posicao = (x, y);
            mostrar_barra(estado, popups, x, y, false);
        }
        Mensagem::Popup(evento) => match evento {
            PopupEvento::Acao(id) => {
                // Botão "Copiar resultado" do balão: copia o que já foi
                // corrigido, sem chamar a API de novo.
                if id == crate::popup::ACAO_COPIAR_RESULTADO {
                    copiar_resultado(estado, popups);
                    return;
                }
                // Botão "Gerar novo texto com IA" do balão:
                if id == crate::popup::ACAO_BALAO_IA {
                    gerar_ia_do_balao(estado, popups, contexto.posicao);
                    return;
                }
                // "⋮" e "✕" mexem no app, não no texto: não passam pelo worker.
                if acao_controle(id, estado, popups) {
                    return;
                }
                // "Revisar avisos…": traz a janela para a frente com a lista; se
                // ainda não houver verificação do texto atual, faz uma agora.
                if id == ACAO_REVISAR {
                    let sem_analise = estado
                        .lock()
                        .map(|c| c.analise.is_empty())
                        .unwrap_or(true);
                    if sem_analise {
                        verificar_selecao(estado, popups, true);
                    } else {
                        if let Ok(mut c) = estado.lock() {
                            c.janela = Some(true);
                            c.log("[Revisão] Abrindo a lista de avisos na janela.");
                        }
                    }
                    return;
                }
                // "Só isto: …": aplica apenas a alternativa escolhida na seta.
                let deslocamento = id.wrapping_sub(ACAO_SUGESTAO_BASE);
                if (deslocamento as usize) < MAX_SUGESTOES {
                    aplicar_sugestao(estado, popups, deslocamento as usize, contexto.foco_editavel);
                    return;
                }
                let (x, y) = contexto.posicao;
                executar(estado, popups, id, contexto.foco_editavel, x, y);
            }
            PopupEvento::Fechado => {}
        },
        Mensagem::Bandeja(acao) => match acao {
            AcaoBandeja::Abrir => {
                if let Ok(mut c) = estado.lock() {
                    c.janela = Some(true);
                }
            }
            AcaoBandeja::AlternarAtivo => {
                let pausou = if let Ok(mut c) = estado.lock() {
                    c.config.ativo = !c.config.ativo;
                    let ativo = c.config.ativo;
                    c.log(format!("[Sistema] Captura {}", if ativo { "ativada" } else { "pausada" }));
                    let _ = c.config.salvar();
                    !ativo
                } else {
                    false
                };
                if pausou {
                    // pausar também fecha um menu que esteja aberto
                    popups.fechar();
                }
            }
            AcaoBandeja::AlternarDeteccao => {
                if let Ok(mut c) = estado.lock() {
                    c.config.deteccao_automatica = !c.config.deteccao_automatica;
                    let ligado = c.config.deteccao_automatica;
                    c.log(format!(
                        "[Sistema] Menu ao selecionar texto {}",
                        if ligado { "ligado" } else { "desligado" }
                    ));
                    let _ = c.config.salvar();
                }
            }
            AcaoBandeja::AlternarAutostart => {
                if let Ok(mut c) = estado.lock() {
                    c.config.iniciar_com_sistema = !c.config.iniciar_com_sistema;
                    let habilitado = c.config.iniciar_com_sistema;
                    match crate::autostart::configurar(habilitado) {
                        Ok(msg) => c.log(format!("[Sistema] {}", msg)),
                        Err(e) => c.log(format!("[Erro Inicialização] {}", e)),
                    }
                    let _ = c.config.salvar();
                }
            }
            AcaoBandeja::Sair => std::process::exit(0),
        },
    }
}

/// Barra de ações mostrada ao selecionar um texto (ou pelo atalho).
///
/// Clique no rótulo = ação principal; clique na seta (ou no "⋮") = flyout de
/// variantes. A seta do "Corrigir" mostra as **alternativas reais** do primeiro
/// aviso encontrado (como no LanguageTool).
pub fn itens_menu(analise: &[Aviso], analisando: bool) -> Vec<ItemBarra> {
    let mut submenu_corrigir = vec![
        (ACAO_CORRIGIR, "Substituir a seleção".to_string()),
        (ACAO_CORRIGIR_COPIAR, "Apenas copiar a correção".to_string()),
    ];

    match primeiro_com_sugestao(analise) {
        Some(aviso) => {
            for (indice, sugestao) in aviso.sugestoes.iter().take(MAX_SUGESTOES).enumerate() {
                submenu_corrigir.push((
                    ACAO_SUGESTAO_BASE + indice as u32,
                    format!("Só isto: {} → {}", resumir(&aviso.trecho), resumir(sugestao)),
                ));
            }
            submenu_corrigir.push((
                ACAO_REVISAR,
                format!("Revisar {} aviso(s) na janela…", analise.len()),
            ));
        }
        None => {
            if analisando {
                submenu_corrigir.push((ACAO_NENHUMA, "Analisando o texto…".to_string()));
            } else {
                submenu_corrigir.push((ACAO_REVISAR, "Abrir a lista de avisos…".to_string()));
            }
        }
    }

    vec![
        ItemBarra::marca(),
        ItemBarra::acao(ACAO_CORRIGIR, "Corrigir", Icone::Corrigir)
            .com_divisor()
            .com_submenu(submenu_corrigir),
        ItemBarra::acao(ACAO_MELHORAR, "", Icone::Estrela)
            .somente_icone()
            .com_seta()
            .com_submenu(vec![
                (ACAO_MELHORAR, "Clareza e gramática".to_string()),
                (ACAO_MELHORAR_FORMAL, "Mais formal".to_string()),
                (ACAO_MELHORAR_SIMPLES, "Mais simples".to_string()),
                (ACAO_MELHORAR_CURTO, "Mais curto".to_string()),
            ]),
        ItemBarra::acao(ACAO_COPIAR, "Copiar", Icone::Copiar).somente_icone(),
        ItemBarra::acao(ACAO_MAIS, "Mais ações", Icone::Mais)
            .somente_icone()
            .com_divisor()
            .com_submenu(vec![
                (ACAO_REVISAR, "Revisar avisos na janela…".to_string()),
                (ACAO_HISTORICO, "Histórico de correções (10 recentes)".to_string()),
                (ACAO_ABRIR_APP, "Configurações do app".to_string()),
                (ACAO_DICIONARIO, "Adicionar seleção ao dicionário".to_string()),
                (ACAO_PAUSAR, "Pausar a captura".to_string()),
                (ACAO_SAIR, "Sair".to_string()),
            ]),
        ItemBarra::acao(ACAO_FECHAR, "Fechar", Icone::Fechar).somente_icone(),
    ]
}

/// Primeiro aviso com alternativa e que não esteja protegido pelo dicionário.
pub fn primeiro_com_sugestao(analise: &[Aviso]) -> Option<&Aviso> {
    analise.iter().find(|a| !a.sugestoes.is_empty() && !a.protegido)
}

/// Mostra a barra com o submenu que já temos e dispara a verificação do texto
/// selecionado (a seta passa a mostrar as alternativas quando ela chega).
fn mostrar_barra(estado: &Estado, popups: &Arc<Popups>, x: i32, y: i32, analisar: bool) {
    let itens = match estado.lock() {
        Ok(c) => itens_menu(&c.analise, c.analisando),
        Err(_) => return,
    };
    popups.barra(x, y, itens);

    if !analisar {
        return;
    }
    let (x_antes, y_antes) = (x, y);
    let estado_worker = Arc::clone(estado);
    let popups_worker = Arc::clone(popups);
    let geracao = match estado.lock() {
        Ok(mut c) => {
            c.analise_geracao += 1;
            c.analisando = true;
            c.analise_geracao
        }
        Err(_) => return,
    };
    let estado_clone = Arc::clone(estado);
    thread::spawn(move || {
        // Só UI Automation: pegar o texto pelo clipboard (Ctrl+C) a cada seleção
        // seria invasivo. Sem texto pelo UIA, a lista fica para o "Revisar avisos…".
        let texto = deteccao::texto_selecionado().filter(|t| !t.trim().is_empty());
        let Some(texto) = texto else {
            if let Ok(mut c) = estado_worker.lock() {
                c.analisando = false;
            }
            return;
        };
        if texto.chars().count() == 0 {
            return;
        }
        let (idioma, dicionario) = match estado_clone.lock() {
            Ok(c) => (
                c.config.idioma.clone(),
                c.config.dicionario.iter().cloned().collect::<std::collections::HashSet<String>>(),
            ),
            Err(_) => return,
        };
        let resultado = crate::correcao::analisar(&texto, &idioma, &dicionario);
        let atual = match estado_worker.lock() {
            Ok(mut c) => {
                // descarta se o usuário já selecionou outro texto
                let atual = c.analise_geracao == geracao;
                if atual {
                    c.analisando = false;
                    match resultado {
                        Ok(avisos) => {
                            c.analise = avisos;
                            c.analise_texto = Some(texto.clone());
                            let total = c.analise.len();
                            let aplicaveis = crate::correcao::contar_aplicaveis(&c.analise);
                            c.log(format!(
                                "[Revisão] {} aviso(s) em {} caractere(s); {} mudariam o texto.",
                                total,
                                texto.chars().count(),
                                aplicaveis
                            ));
                        }
                        Err(e) => {
                            c.log(format!("[Erro na API] {}", e));
                            c.analise.clear();
                            c.analise_texto = None;
                        }
                    }
                }
                atual
            }
            Err(_) => return,
        };
        if atual && popups_worker.barra_aberta() {
            // atualiza a seta do "Corrigir" com as alternativas reais
            let itens = match estado_worker.lock() {
                Ok(c) => itens_menu(&c.analise, false),
                Err(_) => return,
            };
            popups_worker.barra(x_antes, y_antes, itens);
        }
    });
}

/// Verificação explícita (botão "Verificar novamente"/"Revisar avisos…"): usa
/// a captura completa (UI Automation e, se precisar, Ctrl+C).
fn verificar_selecao(estado: &Estado, popups: &Arc<Popups>, abrir_janela: bool) {
    let (idioma, dicionario) = match estado.lock() {
        Ok(c) => (
            c.config.idioma.clone(),
            c.config.dicionario.iter().cloned().collect::<std::collections::HashSet<String>>(),
        ),
        Err(_) => return,
    };
    if let Ok(mut c) = estado.lock() {
        c.analisando = true;
        c.analise_geracao += 1;
    }
    let _ = popups;
    let estado_worker = Arc::clone(estado);
    thread::spawn(move || {
        let texto = match capturar_selecao(&estado_worker) {
            Some(t) if !t.trim().is_empty() => t,
            _ => {
                if let Ok(mut c) = estado_worker.lock() {
                    c.analisando = false;
                    c.log("[Revisão] Nenhum texto selecionado para verificar.");
                }
                return;
            }
        };
        let resultado = crate::correcao::analisar(&texto, &idioma, &dicionario);
        if let Ok(mut c) = estado_worker.lock() {
            c.analisando = false;
            match resultado {
                Ok(avisos) => {
                    let total = avisos.len();
                    let aplicaveis = crate::correcao::contar_aplicaveis(&avisos);
                    c.analise = avisos;
                    c.analise_texto = Some(texto.clone());
                    c.log(format!(
                        "[Revisão] {} aviso(s) em \"{}\"; {} mudariam o texto.",
                        total,
                        resumir(&texto),
                        aplicaveis
                    ));
                }
                Err(e) => {
                    c.log(format!("[Erro na API] {}", e));
                    c.analise.clear();
                    c.analise_texto = None;
                }
            }
            if abrir_janela {
                c.janela = Some(true);
            }
        }
    });
}

/// Aplica **apenas** a alternativa escolhida na seta do "Corrigir".
fn aplicar_sugestao(estado: &Estado, popups: &Arc<Popups>, indice: usize, editavel: bool) {
    {
        let mut c = match estado.lock() {
            Ok(c) => c,
            Err(_) => return,
        };
        if c.ocupado {
            return;
        }
        c.ocupado = true;
    }

    let estado_worker = Arc::clone(estado);
    let popups_worker = Arc::clone(popups);
    thread::spawn(move || {
        let (aviso, texto_analisado) = match estado_worker.lock() {
            Ok(c) => (
                c.analise
                    .iter()
                    .find(|a| !a.sugestoes.is_empty() && !a.protegido)
                    .cloned(),
                c.analise_texto.clone(),
            ),
            Err(_) => {
                return;
            }
        };

        let resultado = (|| -> Result<String, ()> {
            let Some(mut aviso) = aviso else {
                logar(&estado_worker, "[Aviso] Nenhuma alternativa disponível; use \"Corrigir\".");
                return Err(());
            };
            if indice >= aviso.sugestoes.len() {
                logar(&estado_worker, "[Aviso] Alternativa fora da lista; use \"Corrigir\".");
                return Err(());
            }
            aviso.escolhida = indice;

            let texto = match capturar_selecao(&estado_worker) {
                Some(t) if !t.trim().is_empty() => t,
                _ => {
                    logar(&estado_worker, "[Aviso] Nenhum texto selecionado.");
                    return Err(());
                }
            };

            let final_texto = if Some(texto.clone()) == texto_analisado {
                crate::correcao::aplicar_avisos(&texto, &[aviso.clone()])
            } else {
                // a seleção mudou depois da análise: troca pelo trecho mesmo assim
                logar(
                    &estado_worker,
                    "[Revisão] A seleção mudou desde a análise; aplicando a alternativa pelo trecho.",
                );
                trocar_trecho(&texto, &aviso)
            };

            if final_texto == texto {
                logar(&estado_worker, "[Revisão] A alternativa não mudou o texto.");
                return Ok(final_texto);
            }

            entregar(&estado_worker, popups_worker.as_ref(), editavel, &final_texto)?;
            if let Ok(mut c) = estado_worker.lock() {
                c.historico.adicionar(&texto, &final_texto, "Alternativa de Correção");
                c.analise.clear();
                c.analise_texto = None;
            }
            Ok(final_texto)
        })();

        if let (Ok(mut c), Ok(texto)) = (estado_worker.lock(), resultado) {
            c.resultado = Some(texto);
        }
        if let Ok(mut c) = estado_worker.lock() {
            c.ocupado = false;
        }
    });
}

/// Troca a primeira ocorrência do trecho do aviso pela alternativa escolhida.
fn trocar_trecho(texto: &str, aviso: &Aviso) -> String {
    let Some(sugestao) = aviso.sugestao() else {
        return texto.to_string();
    };
    match texto.find(&aviso.trecho) {
        Some(posicao_bytes) => {
            let mut saida = String::with_capacity(texto.len() + sugestao.len());
            saida.push_str(&texto[..posicao_bytes]);
            saida.push_str(sugestao);
            saida.push_str(&texto[posicao_bytes + aviso.trecho.len()..]);
            saida
        }
        None => texto.to_string(),
    }
}

/// Ações que não tocam no texto ("⋮" e "✕").
fn acao_controle(id: u32, estado: &Estado, popups: &Arc<Popups>) -> bool {
    match id {
        ACAO_FECHAR | ACAO_MAIS => {
            popups.fechar();
            true
        }
        ACAO_HISTORICO | ACAO_ABRIR_APP => {
            if let Ok(mut c) = estado.lock() {
                c.janela = Some(true);
            }
            true
        }
        ACAO_PAUSAR => {
            alternar_captura(estado, popups);
            true
        }
        ACAO_DICIONARIO => {
            adicionar_ao_dicionario(estado, popups);
            true
        }
        ACAO_SAIR => std::process::exit(0),
        _ => false,
    }
}

/// Copia o último resultado já obtido (botão do balão).
fn copiar_resultado(estado: &Estado, popups: &Arc<Popups>) {
    let texto = match estado.lock() {
        Ok(c) => c.resultado.clone(),
        Err(_) => None,
    };
    match texto {
        Some(texto) => match crate::aplicacao::copiar_para_clipboard(&texto) {
            Ok(()) => logar(estado, format!("Resultado copiado: {}", resumir(&texto))),
            Err(e) => logar(estado, format!("[Erro no clipboard] {}", e)),
        },
        None => logar(estado, "[Aviso] Não há resultado para copiar."),
    }
    popups.fechar();
}

/// Gera novo texto com IA a partir do conteúdo exibido no balão flutuante.
fn gerar_ia_do_balao(estado: &Estado, popups: &Arc<Popups>, pos: (i32, i32)) {
    let (config, texto) = match estado.lock() {
        Ok(c) => (c.config.clone(), c.resultado.clone().unwrap_or_default()),
        Err(_) => return,
    };
    if texto.trim().is_empty() {
        logar(estado, "[Aviso] Nenhum texto disponível para gerar com IA.");
        return;
    }
    if !config.ia_configurada() {
        logar(estado, "[IA] Configure a IA na aba Configurações antes de usar.");
        popups.balao(pos.0, pos.1, "Configure a URL e modelo de IA nas Opções do app.".to_string());
        return;
    }

    logar(estado, format!("[IA] Gerando novo texto para o balão: {}", resumir(&texto)));
    popups.balao(pos.0, pos.1, "⏳ Gerando novo texto com IA...".to_string());

    let estado_clone = Arc::clone(estado);
    let popups_clone = Arc::clone(popups);
    std::thread::spawn(move || {
        match crate::ia::melhorar(&texto, &config, crate::ia::Estilo::Padrao) {
            Ok(novo_texto) => {
                if let Ok(mut c) = estado_clone.lock() {
                    c.resultado = Some(novo_texto.clone());
                    c.historico.adicionar(&texto, &novo_texto, "IA (Balão)");
                    c.log(format!("[IA] Novo texto gerado no balão: {}", resumir(&novo_texto)));
                }
                let _ = crate::aplicacao::copiar_para_clipboard(&novo_texto);
                popups_clone.balao(pos.0, pos.1, novo_texto);
            }
            Err(e) => {
                logar(&estado_clone, format!("[Erro IA] Falha ao gerar novo texto: {}", e));
                popups_clone.balao(pos.0, pos.1, format!("Falha na IA:\n{}", e));
            }
        }
    });
}

/// Liga/desliga a captura (mesmo efeito do item da bandeja).
fn alternar_captura(estado: &Estado, popups: &Arc<Popups>) {
    let pausou = if let Ok(mut c) = estado.lock() {
        c.config.ativo = !c.config.ativo;
        let ativo = c.config.ativo;
        c.log(format!("[Sistema] Captura {}", if ativo { "ativada" } else { "pausada" }));
        let _ = c.config.salvar();
        !ativo
    } else {
        false
    };
    if pausou {
        popups.fechar();
    }
}

/// Guarda a seleção atual no dicionário pessoal (em minúsculas, como na
/// comparação feita pela correção).
fn adicionar_ao_dicionario(estado: &Estado, popups: &Arc<Popups>) {
    let texto = match deteccao::texto_selecionado() {
        Some(t) => t,
        None => match crate::aplicacao::copiar_selecao(estado) {
            Ok(t) => t,
            Err(e) => {
                logar(estado, format!("[Erro ao capturar seleção] {}", e));
                return;
            }
        },
    };
    let selecao = texto.trim();
    if selecao.is_empty() {
        logar(estado, "[Aviso] Nada selecionado para adicionar ao dicionário.");
        return;
    }
    // o dicionário protege palavras soltas: com um trecho inteiro o palpite
    // seria inútil (e o usuário não veria de onde veio).
    if selecao.split_whitespace().count() > 1 {
        logar(
            estado,
            "[Dicionário] Selecione apenas uma palavra para adicionar ao dicionário.",
        );
        return;
    }
    let palavra = selecao
        .trim_matches(|c: char| !c.is_alphanumeric())
        .to_lowercase();
    if palavra.is_empty() {
        logar(estado, "[Dicionário] A seleção não tem uma palavra válida.");
        return;
    }
    if let Ok(mut c) = estado.lock() {
        if c.config.dicionario.iter().any(|p| p == &palavra) {
            c.log(format!("[Dicionário] \"{}\" já estava cadastrada.", palavra));
        } else {
            c.config.dicionario.push(palavra.clone());
            let _ = c.config.salvar();
            c.log(format!("[Dicionário] \"{}\" não será mais corrigida.", palavra));
        }
    }
    popups.fechar();
}

fn foco_e_nosso() -> bool {
    deteccao::analisar_foco().map(|f| f.pid == std::process::id()).unwrap_or(false)
}

pub fn posicao_cursor() -> (i32, i32) {
    #[cfg(windows)]
    {
        use windows::Win32::Foundation::POINT;
        use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
        let mut p = POINT::default();
        unsafe {
            let _ = GetCursorPos(&mut p);
        }
        (p.x, p.y)
    }
    #[cfg(not(windows))]
    {
        (200, 200)
    }
}

/// Ações que realmente mexem no texto.
fn eh_acao_de_texto(acao: u32) -> bool {
    matches!(
        acao,
        ACAO_CORRIGIR
            | ACAO_CORRIGIR_COPIAR
            | ACAO_COPIAR
            | ACAO_MELHORAR
            | ACAO_MELHORAR_FORMAL
            | ACAO_MELHORAR_SIMPLES
            | ACAO_MELHORAR_CURTO
    )
}

fn eh_acao_de_ia(acao: u32) -> bool {
    matches!(
        acao,
        ACAO_MELHORAR | ACAO_MELHORAR_FORMAL | ACAO_MELHORAR_SIMPLES | ACAO_MELHORAR_CURTO
    )
}

fn executar(estado: &Estado, popups: &Arc<Popups>, acao: u32, editavel: bool, _x: i32, _y: i32) {
    // Rede de segurança: um id desconhecido nunca deve virar uma correção.
    if !eh_acao_de_texto(acao) {
        logar(estado, format!("[Aviso] Ação {} ignorada.", acao));
        return;
    }
    {
        let mut c = match estado.lock() {
            Ok(c) => c,
            Err(_) => return,
        };
        if c.ocupado {
            return;
        }
        c.ocupado = true;
    }

    let estado_worker = Arc::clone(estado);
    let popups_worker = Arc::clone(popups);
    thread::spawn(move || {
        let resultado = executar_acao(&estado_worker, acao, editavel, popups_worker.as_ref());
        if let (Ok(mut c), Ok(texto)) = (estado_worker.lock(), resultado) {
            c.resultado = Some(texto);
        }
        if let Ok(mut c) = estado_worker.lock() {
            c.ocupado = false;
        }
    });
}

/// Captura a seleção, aplica a ação e devolve o texto final.
fn executar_acao(
    estado: &Estado,
    acao: u32,
    editavel: bool,
    popups: &Popups,
) -> Result<String, ()> {
    let (idioma, dicionario, config) = match estado.lock() {
        Ok(c) => (
            c.config.idioma.clone(),
            c.config.dicionario.iter().cloned().collect::<HashSet<String>>(),
            c.config.clone(),
        ),
        Err(_) => return Err(()),
    };

    let texto = match capturar_selecao(estado) {
        Some(t) if !t.trim().is_empty() => t,
        _ => {
            logar(estado, "[Aviso] Nenhum texto selecionado.");
            return Err(());
        }
    };

    // 1) cópia crua da seleção: nenhuma chamada externa.
    if acao == ACAO_COPIAR {
        if let Err(e) = crate::aplicacao::copiar_para_clipboard(&texto) {
            logar(estado, format!("[Erro no clipboard] {}", e));
            return Err(());
        }
        logar(estado, format!("Seleção copiada: {}", resumir(&texto)));
        return Ok(texto);
    }

    let final_texto = if eh_acao_de_ia(acao) {
        let estilo = match acao {
            ACAO_MELHORAR_FORMAL => crate::ia::Estilo::Formal,
            ACAO_MELHORAR_SIMPLES => crate::ia::Estilo::Simples,
            ACAO_MELHORAR_CURTO => crate::ia::Estilo::Curto,
            _ => crate::ia::Estilo::Padrao,
        };
        match crate::ia::melhorar(&texto, &config, estilo) {
            Ok(t) => t,
            Err(e) => {
                logar(estado, format!("[Erro na IA] {}", e));
                let (x, y) = posicao_cursor();
                popups.balao(x, y, format!("Falha na IA:\n{}", e));
                return Err(());
            }
        }
    } else {
        match crate::correcao::corrigir_via_api(&texto, &idioma, &dicionario) {
            Ok(t) => t,
            Err(e) => {
                if config.ia_configurada() {
                    logar(
                        estado,
                        format!("[Aviso API] Falha na rede ({}); acionando IA personalizada como fallback...", e),
                    );
                    match crate::ia::corrigir_com_ia(&texto, &config) {
                        Ok(t) => t,
                        Err(e_ia) => {
                            logar(estado, format!("[Erro na API e IA] API: {} | IA: {}", e, e_ia));
                            return Err(());
                        }
                    }
                } else {
                    logar(estado, format!("[Erro na API] {}", e));
                    return Err(());
                }
            }
        }
    };

    // 2) corrigir sem tocar no campo ("Apenas copiar a correção").
    if acao == ACAO_CORRIGIR_COPIAR {
        if let Err(e) = crate::aplicacao::copiar_para_clipboard(&final_texto) {
            logar(estado, format!("[Erro no clipboard] {}", e));
            return Err(());
        }
        logar(
            estado,
            format!("Correção copiada (campo intacto): {}", resumir(&final_texto)),
        );
        if let Ok(mut c) = estado.lock() {
            c.historico.adicionar(&texto, &final_texto, "Cópia da Correção");
        }
        return Ok(final_texto);
    }

    if final_texto == texto {
        logar(
            estado,
            if eh_acao_de_ia(acao) {
                "A IA devolveu o mesmo texto; nada foi alterado."
            } else {
                "Resultado: Nenhum erro encontrado."
            },
        );
        return Ok(final_texto);
    }

    entregar(estado, popups, editavel, &final_texto)?;

    let tipo = if eh_acao_de_ia(acao) {
        "Melhoria (IA)"
    } else {
        "Correção Ortográfica"
    };

    if let Ok(mut c) = estado.lock() {
        c.historico.adicionar(&texto, &final_texto, tipo);
        c.janela = None;
    }
    Ok(final_texto)
}

/// Entrega o texto final: cola no campo (quando é editável) ou copia e mostra
/// o balão (se ativado). Usado pelo "Corrigir", pelo "Melhorar" e pela seta com alternativas.
fn entregar(estado: &Estado, popups: &Popups, editavel: bool, final_texto: &str) -> Result<(), ()> {
    let balao_ativo = estado.lock().map(|c| c.config.balao_ativo).unwrap_or(true);

    if editavel && Popups::suportado() {
        match crate::aplicacao::substituir_selecao(estado, final_texto) {
            Ok(true) => logar(estado, format!("Aplicado no campo: {}", resumir(final_texto))),
            Ok(false) => {
                logar(estado, "Campo não aceitou a colagem; resultado copiado para a área de transferência.");
                let _ = crate::aplicacao::copiar_para_clipboard(final_texto);
                if balao_ativo {
                    let (x, y) = posicao_cursor();
                    popups.balao(x, y, final_texto.to_string());
                }
            }
            Err(e) => logar(estado, format!("[Erro] {}", e)),
        }
        return Ok(());
    }

    if let Err(e) = crate::aplicacao::copiar_para_clipboard(final_texto) {
        logar(estado, format!("[Erro no clipboard] {}", e));
        return Err(());
    }
    logar(
        estado,
        format!("Campo não editável; copiado: {}", resumir(final_texto)),
    );
    if balao_ativo {
        let (x, y) = posicao_cursor();
        popups.balao(x, y, final_texto.to_string());
    }
    Ok(())
}

/// Pega o texto selecionado: primeiro pelo UI Automation (não mexe no
/// clipboard); se não der, cai no Ctrl+C clássico.
fn capturar_selecao(estado: &Estado) -> Option<String> {
    if let Some(texto) = deteccao::texto_selecionado() {
        if !texto.trim().is_empty() {
            return Some(texto);
        }
    }
    match crate::aplicacao::copiar_selecao(estado) {
        Ok(t) if !t.trim().is_empty() => Some(t),
        Ok(_) => None,
        Err(e) => {
            logar(estado, format!("[Erro ao capturar seleção] {}", e));
            None
        }
    }
}

fn resumir(texto: &str) -> String {
    let limpo: String = texto.split_whitespace().collect::<Vec<_>>().join(" ");
    if limpo.chars().count() > 120 {
        limpo.chars().take(120).collect::<String>() + "..."
    } else {
        limpo
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn aviso(trecho: &str, sugestoes: &[&str]) -> Aviso {
        Aviso {
            offset: 0,
            length: trecho.chars().count(),
            trecho: trecho.to_string(),
            mensagem: String::new(),
            regra: "REGRA".to_string(),
            categoria: "TYPOS".to_string(),
            tipo: "misspelling".to_string(),
            sugestoes: sugestoes.iter().map(|s| s.to_string()).collect(),
            escolhida: 0,
            aceito: true,
            protegido: false,
        }
    }

    fn submenu_corrigir(itens: &[ItemBarra]) -> Vec<(u32, String)> {
        itens
            .iter()
            .find(|i| i.id == ACAO_CORRIGIR)
            .expect("item Corrigir")
            .submenu
            .clone()
    }

    #[test]
    fn a_seta_mostra_as_alternativas_reais_da_api() {
        let avisos = vec![aviso("programaçao", &["programação", "programadão"])];
        let submenu = submenu_corrigir(&itens_menu(&avisos, false));
        let rotulos: Vec<&str> = submenu.iter().map(|(_, r)| r.as_str()).collect();
        assert!(rotulos.contains(&"Só isto: programaçao → programação"), "{:?}", rotulos);
        assert!(rotulos.contains(&"Só isto: programaçao → programadão"), "{:?}", rotulos);
        assert!(submenu.iter().any(|(id, _)| *id == ACAO_SUGESTAO_BASE + 1));
        assert!(rotulos.iter().any(|r| r.starts_with("Revisar 1 aviso")));
        // as duas ações de sempre continuam lá
        assert!(submenu.iter().any(|(id, _)| *id == ACAO_CORRIGIR));
        assert!(submenu.iter().any(|(id, _)| *id == ACAO_CORRIGIR_COPIAR));
    }

    #[test]
    fn a_seta_avisa_enquanto_esta_analisando() {
        let submenu = submenu_corrigir(&itens_menu(&[], true));
        assert!(submenu
            .iter()
            .any(|(id, r)| *id == ACAO_NENHUMA && r.contains("Analisando")));

        let submenu = submenu_corrigir(&itens_menu(&[], false));
        assert!(submenu
            .iter()
            .any(|(id, r)| *id == ACAO_REVISAR && r.contains("lista de avisos")));
    }

    #[test]
    fn aviso_protegido_pelo_dicionario_nao_vai_para_a_seta() {
        let mut protegido = aviso("Rust", &["Rush"]);
        protegido.protegido = true;
        let avisos = vec![protegido, aviso("vc", &["você"])];
        let submenu = submenu_corrigir(&itens_menu(&avisos, false));
        let rotulos: Vec<&str> = submenu.iter().map(|(_, r)| r.as_str()).collect();
        assert!(rotulos.iter().any(|r| r.contains("vc → você")), "{:?}", rotulos);
        assert!(!rotulos.iter().any(|r| r.contains("Rush")), "{:?}", rotulos);
    }

    #[test]
    fn troca_pelo_trecho_quando_a_selecao_mudou_desde_a_analise() {
        let aviso = aviso("programaçao", &["programação"]);
        assert_eq!(
            trocar_trecho("Eu gosto de programaçao.", &aviso),
            "Eu gosto de programação."
        );
        assert_eq!(trocar_trecho("sem o trecho aqui", &aviso), "sem o trecho aqui");
    }

    #[test]
    fn primeiro_com_sugestao_ignora_o_protegido() {
        let mut protegido = aviso("Rust", &["Rush"]);
        protegido.protegido = true;
        let avisos = vec![protegido, aviso("vc", &["você"])];
        assert_eq!(primeiro_com_sugestao(&avisos).map(|a| a.trecho.as_str()), Some("vc"));
        assert!(primeiro_com_sugestao(&[]).is_none());
    }
}
