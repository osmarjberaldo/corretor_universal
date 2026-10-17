#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // Oculta o console no Windows em modo release

mod aplicacao;
mod autostart;
mod config;
mod correcao;
mod deteccao;
mod historico;
mod ia;
mod motor;
mod popup;
mod tray;

use config::Config;
use eframe::egui;
use global_hotkey::{
    hotkey::{Code, HotKey, Modifiers},
    GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState,
};
use motor::{Compartilhado, Estado, Mensagem};
use std::collections::HashSet;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
enum AbaApp {
    Historico,
    TesteAvisos,
    Configuracoes,
    Dicionario,
    Logs,
}

struct CorretorApp {
    estado: Estado,
    canal: std::sync::mpsc::Sender<Mensagem>,
    hotkey_manager: Option<GlobalHotKeyManager>,
    hotkey_atual: Option<HotKey>,
    hotkey_id: Arc<AtomicU32>,
    bandeja: Option<tray::Bandeja>,
    nova_palavra: String,
    texto_teste: String,
    resultado_teste: Option<String>,
    saindo: bool,
    aba_ativa: AbaApp,
    reiniciar_apos_salvar: bool,
    mostrar_modal_fechar: bool,
    lembrar_escolha_fechar: bool,
}

impl CorretorApp {
    fn new(
        cc: &eframe::CreationContext<'_>,
        estado: Estado,
        canal: std::sync::mpsc::Sender<Mensagem>,
        hotkey_manager: Option<GlobalHotKeyManager>,
        hotkey_atual: Option<HotKey>,
        hotkey_id: Arc<AtomicU32>,
        bandeja: Option<tray::Bandeja>,
    ) -> Self {
        if let Ok(mut c) = estado.lock() {
            c.ctx = Some(cc.egui_ctx.clone());
        }
        Self {
            estado,
            canal,
            hotkey_manager,
            hotkey_atual,
            hotkey_id,
            bandeja,
            nova_palavra: String::new(),
            texto_teste: "Eu gosto de programaçao em Rust e vc sabe disso.".to_string(),
            resultado_teste: None,
            saindo: false,
            aba_ativa: AbaApp::Historico,
            reiniciar_apos_salvar: true,
            mostrar_modal_fechar: false,
            lembrar_escolha_fechar: false,
        }
    }

    fn recarregar_atalho(&mut self, config_atalho: &config::AtalhoConfig) {
        if let Some(manager) = &mut self.hotkey_manager {
            if let Some(antigo) = self.hotkey_atual.take() {
                let _ = manager.unregister(antigo);
            }
            if let Some(novo) = converter_atalho(config_atalho) {
                let id = novo.id();
                if manager.register(novo).is_ok() {
                    self.hotkey_atual = Some(novo);
                    self.hotkey_id.store(id, Ordering::Relaxed);
                } else {
                    self.hotkey_id.store(0, Ordering::Relaxed);
                }
            } else {
                self.hotkey_id.store(0, Ordering::Relaxed);
            }
        }
    }
}

/// Lista de avisos da última verificação: aceitar, trocar a alternativa,
/// ignorar ou proteger a palavra no dicionário — um a um.
fn painel_avisos(
    ui: &mut egui::Ui,
    est: &mut Compartilhado,
    canal: &std::sync::mpsc::Sender<Mensagem>,
) {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Avisos da verificação").strong());
                if est.analisando {
                    ui.label("analisando...");
                }
                match &est.analise_texto {
                    Some(texto) => {
                        ui.label(egui::RichText::new(format!("em \"{}\"", resumo_gui(texto))).weak());
                    }
                    None => {
                        ui.label(egui::RichText::new("(nenhuma verificação ainda)").weak());
                    }
                }
            });

            if est.analise.is_empty() {
                ui.label("Selecione um texto (ou use o atalho) e clique em \"Verificar seleção\".");
            }

            let mut proteger: Option<usize> = None;
            egui::ScrollArea::vertical()
                .id_salt("avisos_scroll")
                .max_height(190.0)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    for (indice, aviso) in est.analise.iter_mut().enumerate() {
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut aviso.aceito, "")
                                .on_hover_text("Desmarque para deixar este aviso sem correção");
                            ui.label(egui::RichText::new(aviso.rotulo()).strong());
                            if ui.small_button("Ignorar").clicked() {
                                aviso.aceito = false;
                            }
                            if ui
                                .small_button("Dicionário")
                                .on_hover_text("Nunca mais corrigir esta palavra")
                                .clicked()
                            {
                                proteger = Some(indice);
                            }
                        });
                        if !aviso.sugestoes.is_empty() {
                            ui.horizontal(|ui| {
                                ui.add_space(24.0);
                                for (alternativa, sugestao) in
                                    aviso.sugestoes.iter().take(6).enumerate()
                                {
                                    if ui
                                        .selectable_label(aviso.escolhida == alternativa, sugestao)
                                        .clicked()
                                    {
                                        aviso.escolhida = alternativa;
                                        aviso.aceito = true;
                                    }
                                }
                            });
                        }
                        ui.label(
                            egui::RichText::new(format!(
                                "{} · {} · {}",
                                aviso.regra, aviso.categoria, aviso.mensagem
                            ))
                            .weak(),
                        );
                        ui.separator();
                    }
                });

            if let Some(indice) = proteger {
                // sem manter o empréstimo de `est.analise` enquanto mexe na config
                let palavra = est
                    .analise
                    .get(indice)
                    .map(|a| a.trecho.trim().to_lowercase())
                    .unwrap_or_default();
                let nova = !palavra.is_empty() && !est.config.dicionario.contains(&palavra);
                if nova {
                    est.config.dicionario.push(palavra.clone());
                    let _ = est.config.salvar();
                }
                if let Some(aviso) = est.analise.get_mut(indice) {
                    aviso.protegido = true;
                    aviso.aceito = false;
                }
                if nova {
                    est.log(format!(
                        "[Dicionário] '{}' adicionada às exceções (não será mais corrigida).",
                        palavra
                    ));
                }
            }

            let aplicaveis = correcao::contar_aplicaveis(&est.analise);
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        aplicaveis > 0,
                        egui::Button::new(format!(
                            "Aplicar {} aviso(s) e copiar o resultado",
                            aplicaveis
                        )),
                    )
                    .clicked()
                {
                    if let Some(analisado) = est.analise_texto.clone() {
                        let final_texto = correcao::aplicar_avisos(&analisado, &est.analise);
                        match aplicacao::copiar_para_clipboard(&final_texto) {
                            Ok(()) => {
                                est.log(format!(
                                    "[Revisão] {} aviso(s) aplicado(s); resultado copiado: {}",
                                    aplicaveis,
                                    resumo_gui(&final_texto)
                                ));
                                est.resultado = Some(final_texto);
                            }
                            Err(e) => est.log(format!("[Erro no clipboard] {}", e)),
                        }
                    }
                }
                if ui
                    .button("Verificar seleção")
                    .on_hover_text("Verifica de novo o texto selecionado no outro aplicativo")
                    .clicked()
                {
                    let _ = canal.send(Mensagem::Reanalisar);
                }
                if ui.button("Marcar todos").clicked() {
                    for aviso in est.analise.iter_mut() {
                        if !aviso.protegido {
                            aviso.aceito = true;
                        }
                    }
                }
                if ui.button("Limpar lista").clicked() {
                    est.analise.clear();
                    est.analise_texto = None;
                }
            });
        });
    }

impl CorretorApp {
    fn janela(&mut self, ctx: &egui::Context) {
        // Trata o clique no botão fechar (X) da janela.
        let pedido_fechar = ctx.input(|i| i.viewport().close_requested());
        if pedido_fechar && !self.saindo {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            let acao = self
                .estado
                .lock()
                .ok()
                .map(|c| c.config.acao_ao_fechar)
                .unwrap_or_default();
            match acao {
                config::AcaoFechar::Minimizar => {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                }
                config::AcaoFechar::Encerrar => {
                    self.saindo = true;
                    std::process::exit(0);
                }
                config::AcaoFechar::Perguntar => {
                    self.mostrar_modal_fechar = true;
                }
            }
        }

        // Pedido de reabertura vindo da bandeja/atalho.
        let pedido = self.estado.lock().ok().and_then(|mut c| c.janela.take());
        if pedido == Some(true) {
            self.mostrar_modal_fechar = false;
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            ctx.request_repaint();
        }
    }
}

impl eframe::App for CorretorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.janela(ctx);

        let mut est = match self.estado.lock() {
            Ok(e) => e,
            Err(_) => return,
        };
        let canal = self.canal.clone();
        let mut novo_atalho_config: Option<config::AtalhoConfig> = None;

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Corretor Universal");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let atalho = est.config.atalho.texto_exibicao();
                    ui.label(egui::RichText::new(format!("Atalho: {}", atalho)).weak());
                });
            });
            ui.add_space(2.0);

            // Barra superior de status
            ui.horizontal(|ui| {
                let (texto, cor) = if est.config.ativo {
                    ("Captura ATIVA", egui::Color32::from_rgb(40, 180, 99))
                } else {
                    ("Captura PAUSADA", egui::Color32::from_rgb(231, 76, 60))
                };
                ui.colored_label(cor, egui::RichText::new(texto).strong());
                if ui.button(if est.config.ativo { "Pausar" } else { "Ativar" }).clicked() {
                    est.config.ativo = !est.config.ativo;
                    let _ = est.config.salvar();
                }
                ui.add_enabled(
                    deteccao::disponivel(),
                    egui::Checkbox::new(&mut est.config.deteccao_automatica, "Menu ao selecionar"),
                );
                if est.config.deteccao_automatica != est.deteccao_anterior {
                    est.deteccao_anterior = est.config.deteccao_automatica;
                    let _ = est.config.salvar();
                }
                ui.separator();
                ui.label("Idioma:");
                egui::ComboBox::from_id_salt("cb_idioma")
                    .selected_text(&est.config.idioma)
                    .show_ui(ui, |ui| {
                        for (codigo, rotulo) in [
                            ("pt-BR", "Português (BR)"),
                            ("pt-PT", "Português (PT)"),
                            ("en-US", "Inglês (US)"),
                            ("es", "Espanhol"),
                        ] {
                            if ui.selectable_value(&mut est.config.idioma, codigo.to_string(), rotulo).changed() {
                                let _ = est.config.salvar();
                            }
                        }
                    });
            });

            ui.add_space(4.0);
            ui.separator();

            // Abas de navegação
            ui.horizontal(|ui| {
                let n_hist = est.historico.itens.len();
                let txt_hist = format!("🕒 Histórico ({}/10)", n_hist);
                if ui.selectable_label(self.aba_ativa == AbaApp::Historico, egui::RichText::new(txt_hist).strong()).clicked() {
                    self.aba_ativa = AbaApp::Historico;
                }
                if ui.selectable_label(self.aba_ativa == AbaApp::TesteAvisos, "✍️ Teste & Avisos").clicked() {
                    self.aba_ativa = AbaApp::TesteAvisos;
                }
                if ui.selectable_label(self.aba_ativa == AbaApp::Configuracoes, "⚙️ IA & Opções").clicked() {
                    self.aba_ativa = AbaApp::Configuracoes;
                }
                let n_dic = est.config.dicionario.len();
                let txt_dic = format!("📖 Dicionário ({})", n_dic);
                if ui.selectable_label(self.aba_ativa == AbaApp::Dicionario, txt_dic).clicked() {
                    self.aba_ativa = AbaApp::Dicionario;
                }
                if ui.selectable_label(self.aba_ativa == AbaApp::Logs, "📝 Logs").clicked() {
                    self.aba_ativa = AbaApp::Logs;
                }
            });
            ui.separator();
            ui.add_space(4.0);

            // Conteúdo da aba selecionada
            match self.aba_ativa {
                AbaApp::Historico => {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Histórico de correções (máximo de 10 recentes):").heading().size(15.0));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if !est.historico.itens.is_empty() {
                                if ui.button("🗑 Limpar Histórico").on_hover_text("Apagar todos os registros do histórico").clicked() {
                                    est.historico.limpar();
                                }
                            }
                        });
                    });
                    ui.add_space(4.0);

                    if est.historico.itens.is_empty() {
                        ui.add_space(30.0);
                        ui.vertical_centered(|ui| {
                            ui.label(egui::RichText::new("Nenhum texto no histórico ainda.").size(15.0).italics());
                            ui.add_space(6.0);
                            ui.label(egui::RichText::new("Sempre que você corrigir ou melhorar um texto com o app\n(pelo menu flutuante, atalho Ctrl+Alt+C ou Teste Rápido),\nele será salvo aqui automaticamente até o limite de 10.").weak());
                        });
                    } else {
                        egui::ScrollArea::vertical()
                            .id_salt("scroll_historico")
                            .auto_shrink([false; 2])
                            .show(ui, |ui| {
                                for (idx, item) in est.historico.itens.iter().enumerate() {
                                    ui.group(|ui| {
                                        ui.horizontal(|ui| {
                                            let cor_badge = if item.tipo.contains("IA") {
                                                egui::Color32::from_rgb(155, 89, 182)
                                            } else {
                                                egui::Color32::from_rgb(41, 128, 185)
                                            };
                                            ui.colored_label(cor_badge, egui::RichText::new(format!("#{} [{}]", idx + 1, item.tipo)).strong());
                                            ui.label(egui::RichText::new(&item.data_hora).weak());
                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                if ui.button("📋 Copiar").on_hover_text("Copiar texto corrigido").clicked() {
                                                    let _ = aplicacao::copiar_para_clipboard(&item.corrigido);
                                                }
                                            });
                                        });
                                        ui.add_space(2.0);
                                        ui.horizontal_wrapped(|ui| {
                                            ui.label(egui::RichText::new("Original : ").weak());
                                            ui.label(egui::RichText::new(&item.original).strikethrough().weak());
                                        });
                                        ui.horizontal_wrapped(|ui| {
                                            ui.label(egui::RichText::new("Corrigido: ").strong());
                                            ui.colored_label(
                                                egui::Color32::from_rgb(39, 174, 96),
                                                egui::RichText::new(&item.corrigido).strong(),
                                            );
                                        });
                                    });
                                    ui.add_space(4.0);
                                }
                            });
                    }
                }
                AbaApp::TesteAvisos => {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Design & Ícones Apple Aqua:").strong());
                            if ui
                                .button(
                                    egui::RichText::new("🎯 Testar Barra Flutuante Agora")
                                        .color(egui::Color32::from_rgb(88, 166, 255))
                                        .strong(),
                                )
                                .on_hover_text(
                                    "Abre a barra flutuante na posição do mouse para inspecionar os ícones, os 3 pontos (⋮) e o visual Apple Liquid Glass",
                                )
                                .clicked()
                            {
                                let (cx, cy) = motor::posicao_cursor();
                                let _ = canal.send(motor::Mensagem::TestarBarra { x: cx, y: cy });
                            }
                        });
                        ui.label(
                            egui::RichText::new(
                                "Abra a barra para testar os três pontos (⋮), o ícone gota d'água 'A', copiar, hover e submenus.",
                            )
                            .weak()
                            .small(),
                        );
                    });
                    ui.add_space(4.0);

                    ui.group(|ui| {
                        ui.label(egui::RichText::new("Teste Rápido de Correção:").strong());
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut self.texto_teste)
                                    .desired_width(380.0)
                                    .hint_text("Digite ou cole um texto com erro aqui"),
                            );
                            let clicou = ui.button("Corrigir Agora").clicked();
                            if clicou && !self.texto_teste.trim().is_empty() {
                                let texto = self.texto_teste.clone();
                                let idioma = est.config.idioma.clone();
                                let dicionario = est.config.dicionario.iter().cloned().collect();
                                let config = est.config.clone();
                                match correcao::corrigir_via_api(&texto, &idioma, &dicionario) {
                                    Ok(c) => {
                                        if c != texto {
                                            est.historico.adicionar(&texto, &c, "Teste Rápido");
                                        }
                                        self.resultado_teste = Some(c);
                                    }
                                    Err(e) => {
                                        // Fallback automático para a IA quando a API pública oscila
                                        if config.ia_configurada() {
                                            match ia::corrigir_com_ia(&texto, &config) {
                                                Ok(c) => {
                                                    if c != texto {
                                                        est.historico.adicionar(&texto, &c, "Teste Rápido (IA Fallback)");
                                                    }
                                                    self.resultado_teste = Some(format!("{} (corrigido via IA)", c));
                                                }
                                                Err(e_ia) => {
                                                    self.resultado_teste = Some(format!("[Erro API: {} | Erro IA: {}]", e, e_ia));
                                                }
                                            }
                                        } else {
                                            self.resultado_teste = Some(format!("[Erro na API] {}", e));
                                        }
                                    }
                                }
                            }
                        });
                        if let Some(resultado) = &self.resultado_teste {
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("Resultado:").strong());
                                ui.colored_label(egui::Color32::from_rgb(46, 204, 113), resultado);
                                if ui.small_button("Copiar").clicked() {
                                    let _ = aplicacao::copiar_para_clipboard(resultado);
                                }
                            });
                        }
                    });

                    ui.add_space(4.0);
                    painel_avisos(ui, &mut est, &canal);

                    ui.add_space(4.0);
                    ui.label(egui::RichText::new("Último resultado entregue:").strong());
                    ui.add(
                        egui::TextEdit::multiline(&mut est.resultado.clone().unwrap_or_default())
                            .desired_rows(2)
                            .desired_width(f32::INFINITY)
                            .interactive(false),
                    );
                }
                AbaApp::Configuracoes => {
                    ui.group(|ui| {
                        ui.label(egui::RichText::new("IA personalizada (usada no \"Melhorar com IA\")").strong());
                        egui::Grid::new("grid_ia").num_columns(2).show(ui, |ui| {
                            ui.label("URL (compatível com OpenAI):");
                            ui.add(egui::TextEdit::singleline(&mut est.config.ia_url).desired_width(360.0));
                            ui.end_row();
                            ui.label("Chave da API:");
                            ui.add(egui::TextEdit::singleline(&mut est.config.ia_chave).password(true).desired_width(360.0));
                            ui.end_row();
                            ui.label("Modelo:");
                            ui.add(egui::TextEdit::singleline(&mut est.config.ia_modelo).desired_width(360.0));
                            ui.end_row();
                            ui.label("Instrução:");
                            ui.add(egui::TextEdit::multiline(&mut est.config.ia_prompt).desired_rows(3).desired_width(360.0));
                            ui.end_row();
                        });
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.checkbox(
                                &mut self.reiniciar_apos_salvar,
                                "Reiniciar o aplicativo automaticamente ao salvar dados da IA",
                            );
                        });
                        ui.add_space(2.0);
                        ui.horizontal(|ui| {
                            if ui.button("💾 Salvar IA").clicked() {
                                match est.config.salvar() {
                                    Ok(_) => {
                                        let caminho = Config::caminho();
                                        est.status_ia = format!("Configuração salva em {}", caminho.display());
                                        if self.reiniciar_apos_salvar {
                                            reiniciar_aplicativo();
                                        }
                                    }
                                    Err(e) => est.status_ia = format!("Erro ao salvar: {}", e),
                                }
                            }
                            if ui.button("🔄 Salvar e Reiniciar Agora").on_hover_text("Salva as configurações e reinicia o aplicativo na hora").clicked() {
                                let _ = est.config.salvar();
                                reiniciar_aplicativo();
                            }
                            if ui.button("⚡ Testar IA").clicked() {
                                let config = est.config.clone();
                                let estado = Arc::clone(&self.estado);
                                est.status_ia = "Testando conexão com a IA...".to_string();
                                std::thread::spawn(move || {
                                    let resposta = ia::melhorar(
                                        "hoje eu fui no mercado e comprei maçã",
                                        &config,
                                        ia::Estilo::Padrao,
                                    );
                                    if let Ok(mut c) = estado.lock() {
                                        c.status_ia = match resposta {
                                            Ok(t) => format!("OK: {}", t),
                                            Err(e) => format!("Falhou: {}", e),
                                        };
                                    }
                                });
                            }
                            if !est.status_ia.is_empty() {
                                ui.label(&est.status_ia);
                            }
                        });
                    });

                    ui.add_space(8.0);
                    ui.group(|ui| {
                        ui.label(egui::RichText::new("🚀 Inicialização Automática").strong());
                        let mut autostart = est.config.iniciar_com_sistema;
                        if ui
                            .checkbox(
                                &mut autostart,
                                "Iniciar o Corretor Universal automaticamente com o sistema (Windows / macOS / Linux)",
                            )
                            .changed()
                        {
                            est.config.iniciar_com_sistema = autostart;
                            match autostart::configurar(autostart) {
                                Ok(msg) => {
                                    let _ = est.config.salvar();
                                    est.log(format!("[Inicialização] {}", msg));
                                }
                                Err(e) => {
                                    est.log(format!("[Erro Inicialização] {}", e));
                                }
                            }
                        }
                        let sistema_desc = if cfg!(target_os = "windows") {
                            "Integração nativa com o Registro do Windows (HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run)."
                        } else if cfg!(target_os = "macos") {
                            "Integração nativa com o LaunchAgents do macOS (~/Library/LaunchAgents)."
                        } else {
                            "Integração nativa com o XDG Autostart do Linux (~/.config/autostart)."
                        };
                        ui.label(egui::RichText::new(sistema_desc).small().weak());
                    });

                    ui.add_space(8.0);
                    ui.group(|ui| {
                        ui.label(egui::RichText::new("🪟 Ação ao Clicar no Botão Fechar (✕)").strong());
                        ui.horizontal(|ui| {
                            let mut acao = est.config.acao_ao_fechar;
                            let mut mudou = false;
                            if ui.radio_value(&mut acao, config::AcaoFechar::Perguntar, "Sempre perguntar").changed() {
                                mudou = true;
                            }
                            if ui.radio_value(&mut acao, config::AcaoFechar::Minimizar, "Minimizar para a bandeja (junto ao relógio)").changed() {
                                mudou = true;
                            }
                            if ui.radio_value(&mut acao, config::AcaoFechar::Encerrar, "Encerrar aplicativo").changed() {
                                mudou = true;
                            }
                            if mudou {
                                est.config.acao_ao_fechar = acao;
                                let _ = est.config.salvar();
                                est.log(format!("[Config] Ação ao fechar alterada para: {:?}", acao));
                            }
                        });
                        ui.label(
                            egui::RichText::new(
                                "Define se ao clicar no ✕ da janela o app exibe o aviso para confirmar se deseja fechar de vez ou minimizar para segundo plano nos ícones junto ao relógio.",
                            )
                            .small()
                            .weak(),
                        );
                    });

                    ui.add_space(8.0);
                    let mut atalho_alterado = false;
                    ui.group(|ui| {
                        ui.label(egui::RichText::new("⌨️ Atalho Global do Teclado").strong());
                        ui.label(
                            egui::RichText::new(
                                "Personalize a combinação de teclas para acionar o corretor instantaneamente em qualquer app:",
                            )
                            .weak(),
                        );
                        ui.horizontal(|ui| {
                            atalho_alterado |= ui.checkbox(&mut est.config.atalho.ctrl, "Ctrl").changed();
                            atalho_alterado |= ui
                                .checkbox(
                                    &mut est.config.atalho.alt,
                                    if cfg!(target_os = "macos") { "Option" } else { "Alt" },
                                )
                                .changed();
                            atalho_alterado |= ui.checkbox(&mut est.config.atalho.shift, "Shift").changed();
                            atalho_alterado |= ui
                                .checkbox(
                                    &mut est.config.atalho.meta,
                                    if cfg!(target_os = "macos") { "Cmd" } else { "Win" },
                                )
                                .changed();

                            ui.separator();
                            ui.label("Tecla:");
                            egui::ComboBox::from_id_salt("combo_tecla_atalho")
                                .selected_text(&est.config.atalho.tecla)
                                .show_ui(ui, |ui| {
                                    for tecla in [
                                        "C", "A", "B", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M",
                                        "N", "O", "P", "Q", "R", "S", "T", "U", "V", "W", "X", "Y", "Z",
                                        "1", "2", "3", "4", "5", "6", "7", "8", "9", "0",
                                        "F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9", "F10", "F11", "F12",
                                        "Space",
                                    ] {
                                        if ui.selectable_value(&mut est.config.atalho.tecla, tecla.to_string(), tecla).changed() {
                                            atalho_alterado = true;
                                        }
                                    }
                                });
                        });
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(format!(
                                    "Atalho ativo: {}",
                                    est.config.atalho.texto_exibicao()
                                ))
                                .strong(),
                            );
                            if ui.button("Restaurar padrão").clicked() {
                                est.config.atalho = config::AtalhoConfig::default();
                                atalho_alterado = true;
                            }
                        });
                    });
                    if atalho_alterado {
                        let _ = est.config.salvar();
                        let txt = est.config.atalho.texto_exibicao();
                        est.log(format!("[Atalho] Atalho global alterado para: {}", txt));
                        novo_atalho_config = Some(est.config.atalho.clone());
                    }

                    ui.add_space(8.0);
                    ui.group(|ui| {
                        ui.label(egui::RichText::new("💬 Balão Flutuante de Resultado").strong());
                        let mut balao = est.config.balao_ativo;
                        if ui
                            .checkbox(
                                &mut balao,
                                "Exibir balão flutuante quando o campo não aceitar substituição direta",
                            )
                            .changed()
                        {
                            est.config.balao_ativo = balao;
                            let _ = est.config.salvar();
                            est.log(format!(
                                "[Balão] Balão flutuante {}",
                                if balao { "ativado" } else { "desativado" }
                            ));
                        }
                        ui.label(
                            egui::RichText::new(
                                "Quando ativo, campos de somente leitura (PDFs, navegadores) exibem o balão flutuante \
                                 com opções para copiar o resultado e gerar novo texto com IA diretamente.",
                            )
                            .small()
                            .weak(),
                        );
                    });

                    ui.add_space(8.0);
                    ui.group(|ui| {
                        ui.label(egui::RichText::new("🧠 Contexto da frase e Confirmação").strong());

                        ui.label(
                            egui::RichText::new(
                                "Com a IA configurada acima, o botão \"Corrigir\" primeiro arruma a ortografia \
                                 (LanguageTool) e depois usa a IA para ajustar a coerência/contexto das frases. \
                                 Sem IA configurada, fica só a correção ortográfica do LanguageTool.",
                            )
                            .small()
                            .weak(),
                        );

                        ui.add_space(4.0);
                        let mut confirmar = est.config.confirmar_substituicao;
                        if ui
                            .checkbox(
                                &mut confirmar,
                                "Mostrar prévia do novo texto e pedir confirmação antes de substituir",
                            )
                            .changed()
                        {
                            est.config.confirmar_substituicao = confirmar;
                            let _ = est.config.salvar();
                            est.log(format!(
                                "[Confirmação] Prévia antes de substituir {}",
                                if confirmar { "ativada" } else { "desativada" }
                            ));
                        }
                        ui.label(
                            egui::RichText::new(
                                "Quando ativo, ao clicar em \"Corrigir\" (ou \"Melhorar\") o texto só é substituído \
                                 depois que você conferir o balão de prévia e clicar em \"Confirmar substituição\".",
                            )
                            .small()
                            .weak(),
                        );
                    });

                    ui.add_space(8.0);
                    ui.group(|ui| {
                        ui.label(egui::RichText::new("🛡️ Compatibilidade com outros aplicativos").strong());
                        ui.label(
                            egui::RichText::new(
                                "A barra funciona na maioria dos apps. Em aplicativos executados como administrador, \
                                 o Windows bloqueia a captura e a colagem; reinicie o Corretor como administrador \
                                 para atender também esses casos.",
                            )
                            .small()
                            .weak(),
                        );
                        ui.add_space(2.0);
                        if ui
                            .add_enabled(
                                cfg!(windows),
                                egui::Button::new("🛡️ Reiniciar como administrador"),
                            )
                            .on_hover_text(
                                "Relança o app elevado (UAC) para funcionar em apps abertos como administrador",
                            )
                            .clicked()
                        {
                            match reiniciar_como_admin() {
                                Ok(()) => {}
                                Err(e) => est.log(format!("[Administrador] {}", e)),
                            }
                        }
                    });
                }
                AbaApp::Dicionario => {
                    ui.group(|ui| {
                        ui.label(egui::RichText::new("Dicionário pessoal (palavras ignoradas pelo corretor):").strong());
                        ui.horizontal(|ui| {
                            ui.text_edit_singleline(&mut self.nova_palavra);
                            if ui.button("Adicionar").clicked() && !self.nova_palavra.trim().is_empty() {
                                let palavra = self.nova_palavra.trim().to_lowercase();
                                if !est.config.dicionario.contains(&palavra) {
                                    est.config.dicionario.push(palavra.clone());
                                    est.log(format!("[Dicionário] '{}' adicionada às exceções.", palavra));
                                    let _ = est.config.salvar();
                                }
                                self.nova_palavra.clear();
                            }
                        });
                        ui.add_space(4.0);

                        let mut remover: Option<usize> = None;
                        egui::ScrollArea::vertical()
                            .id_salt("scroll_dicionario")
                            .max_height(250.0)
                            .show(ui, |ui| {
                                if est.config.dicionario.is_empty() {
                                    ui.label("(nenhuma palavra cadastrada)");
                                }
                                for (idx, pal) in est.config.dicionario.iter().enumerate() {
                                    ui.horizontal(|ui| {
                                        ui.label(format!("• {}", pal));
                                        if ui.small_button("Remover").clicked() {
                                            remover = Some(idx);
                                        }
                                    });
                                }
                            });
                        if let Some(idx) = remover {
                            if idx < est.config.dicionario.len() {
                                let p = est.config.dicionario.remove(idx);
                                est.log(format!("[Dicionário] '{}' removida.", p));
                                let _ = est.config.salvar();
                            }
                        }
                    });
                }
                AbaApp::Logs => {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Logs técnicos de eventos:").strong());
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Limpar Logs").clicked() {
                                est.logs.clear();
                            }
                        });
                    });
                    ui.add_space(2.0);
                    egui::ScrollArea::vertical()
                        .id_salt("scroll_logs")
                        .auto_shrink([false; 2])
                        .stick_to_bottom(true)
                        .show(ui, |ui| {
                            for log in &est.logs {
                                ui.monospace(log);
                            }
                        });
                }
            }
        });

        // Diálogo de confirmação ao clicar no botão fechar (X)
        if self.mostrar_modal_fechar {
            let screen_rect = ctx.screen_rect();
            egui::Area::new(egui::Id::new("modal_fechar_backdrop"))
                .fixed_pos(screen_rect.min)
                .order(egui::Order::Foreground)
                .show(ctx, |ui| {
                    ui.allocate_rect(screen_rect, egui::Sense::click());
                    ui.painter().rect_filled(
                        screen_rect,
                        0.0,
                        egui::Color32::from_black_alpha(150),
                    );
                });

            let mut fechar_dialogo = false;
            let mut acao_escolhida: Option<config::AcaoFechar> = None;

            egui::Window::new("Fechar ou Minimizar?")
                .id(egui::Id::new("modal_fechar_janela"))
                .collapsible(false)
                .resizable(false)
                .movable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .order(egui::Order::Tooltip)
                .show(ctx, |ui| {
                    ui.set_width(420.0);
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("❓").size(24.0));
                            ui.vertical(|ui| {
                                ui.label(
                                    egui::RichText::new("O que você deseja fazer?")
                                        .strong()
                                        .size(15.0),
                                );
                                ui.label(
                                    egui::RichText::new(
                                        "Você clicou no botão fechar da janela.",
                                    )
                                    .weak(),
                                );
                            });
                        });

                        ui.add_space(6.0);
                        ui.separator();
                        ui.add_space(6.0);

                        ui.group(|ui| {
                            ui.label(egui::RichText::new("Escolha uma opção:").strong());
                            ui.add_space(4.0);
                            ui.label(
                                egui::RichText::new(
                                    "• Minimizar para a bandeja:\n  O aplicativo continua rodando em segundo plano nos\n  ícones junto ao relógio (área de notificação), pronto para\n  fazer correções e responder aos atalhos.",
                                )
                                .weak(),
                            );
                            ui.add_space(4.0);
                            ui.label(
                                egui::RichText::new(
                                    "• Fechar aplicativo:\n  Encerra o Corretor Universal completamente.",
                                )
                                .weak(),
                            );
                        });

                        ui.add_space(6.0);
                        ui.checkbox(
                            &mut self.lembrar_escolha_fechar,
                            "Lembrar minha decisão (não perguntar novamente)",
                        );

                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            if ui
                                .button(
                                    egui::RichText::new("🕒 Minimizar para a bandeja")
                                        .color(egui::Color32::from_rgb(88, 166, 255))
                                        .strong(),
                                )
                                .on_hover_text("Fica em segundo plano nos ícones junto ao relógio")
                                .clicked()
                            {
                                acao_escolhida = Some(config::AcaoFechar::Minimizar);
                            }

                            if ui
                                .button(
                                    egui::RichText::new("✕ Fechar app")
                                        .color(egui::Color32::from_rgb(231, 76, 60))
                                        .strong(),
                                )
                                .on_hover_text("Encerra totalmente o aplicativo")
                                .clicked()
                            {
                                acao_escolhida = Some(config::AcaoFechar::Encerrar);
                            }

                            if ui
                                .button("Cancelar")
                                .on_hover_text("Volta ao app sem fechar nem minimizar")
                                .clicked()
                            {
                                fechar_dialogo = true;
                            }
                        });
                    });
                });

            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                fechar_dialogo = true;
            }

            if let Some(acao) = acao_escolhida {
                if self.lembrar_escolha_fechar {
                    est.config.acao_ao_fechar = acao;
                    let _ = est.config.salvar();
                }
                self.mostrar_modal_fechar = false;
                match acao {
                    config::AcaoFechar::Minimizar => {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                    }
                    config::AcaoFechar::Encerrar => {
                        self.saindo = true;
                        std::process::exit(0);
                    }
                    config::AcaoFechar::Perguntar => {}
                }
            } else if fechar_dialogo {
                self.mostrar_modal_fechar = false;
            }
        }

        // Mantém o estado da bandeja e da detecção iguais à configuração.
        if let Some(bandeja) = &self.bandeja {
            bandeja.sincronizar(est.config.ativo, est.config.deteccao_automatica, est.config.iniciar_com_sistema);
        }
        deteccao::ligar(est.config.ativo && est.config.deteccao_automatica);
        drop(est);

        if let Some(cfg) = novo_atalho_config {
            self.recarregar_atalho(&cfg);
        }

        ctx.request_repaint_after(Duration::from_millis(150));
    }
}

fn converter_atalho(config: &config::AtalhoConfig) -> Option<HotKey> {
    let mut mods = Modifiers::empty();
    if config.ctrl {
        mods |= Modifiers::CONTROL;
    }
    if config.alt {
        mods |= Modifiers::ALT;
    }
    if config.shift {
        mods |= Modifiers::SHIFT;
    }
    if config.meta {
        mods |= Modifiers::META;
    }

    let code = match config.tecla.to_ascii_uppercase().as_str() {
        "A" => Code::KeyA,
        "B" => Code::KeyB,
        "C" => Code::KeyC,
        "D" => Code::KeyD,
        "E" => Code::KeyE,
        "F" => Code::KeyF,
        "G" => Code::KeyG,
        "H" => Code::KeyH,
        "I" => Code::KeyI,
        "J" => Code::KeyJ,
        "K" => Code::KeyK,
        "L" => Code::KeyL,
        "M" => Code::KeyM,
        "N" => Code::KeyN,
        "O" => Code::KeyO,
        "P" => Code::KeyP,
        "Q" => Code::KeyQ,
        "R" => Code::KeyR,
        "S" => Code::KeyS,
        "T" => Code::KeyT,
        "U" => Code::KeyU,
        "V" => Code::KeyV,
        "W" => Code::KeyW,
        "X" => Code::KeyX,
        "Y" => Code::KeyY,
        "Z" => Code::KeyZ,
        "1" => Code::Digit1,
        "2" => Code::Digit2,
        "3" => Code::Digit3,
        "4" => Code::Digit4,
        "5" => Code::Digit5,
        "6" => Code::Digit6,
        "7" => Code::Digit7,
        "8" => Code::Digit8,
        "9" => Code::Digit9,
        "0" => Code::Digit0,
        "F1" => Code::F1,
        "F2" => Code::F2,
        "F3" => Code::F3,
        "F4" => Code::F4,
        "F5" => Code::F5,
        "F6" => Code::F6,
        "F7" => Code::F7,
        "F8" => Code::F8,
        "F9" => Code::F9,
        "F10" => Code::F10,
        "F11" => Code::F11,
        "F12" => Code::F12,
        "SPACE" | "ESPAÇO" => Code::Space,
        _ => Code::KeyC,
    };

    Some(HotKey::new(if mods.is_empty() { None } else { Some(mods) }, code))
}

fn criar_atalho(
    canal: std::sync::mpsc::Sender<Mensagem>,
    config_atalho: &config::AtalhoConfig,
) -> (Option<GlobalHotKeyManager>, Option<HotKey>, Arc<AtomicU32>) {
    let hotkey_id = Arc::new(AtomicU32::new(0));
    let manager = match GlobalHotKeyManager::new() {
        Ok(m) => m,
        Err(_) => return (None, None, hotkey_id),
    };

    let hotkey = converter_atalho(config_atalho);
    if let Some(hk) = &hotkey {
        let id = hk.id();
        hotkey_id.store(id, Ordering::Relaxed);
        let _ = manager.register(*hk);
    }

    let id_atual = Arc::clone(&hotkey_id);
    GlobalHotKeyEvent::set_event_handler(Some(move |evento: GlobalHotKeyEvent| {
        let atual = id_atual.load(Ordering::Relaxed);
        if atual != 0 && evento.id == atual && evento.state == HotKeyState::Pressed {
            let _ = canal.send(Mensagem::Hotkey);
        }
    }));

    (Some(manager), hotkey, hotkey_id)
}

fn main() -> eframe::Result<()> {
    let _ = std::fs::write("app_debug.log", "Iniciando main...\n");
    std::panic::set_hook(Box::new(|info| {
        let msg = format!("CRASH: {:?}\n", info);
        let _ = std::fs::write("corretor_crash.log", msg);
    }));

    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--self-test") {
        std::process::exit(autoteste(&args));
    }
    if args.iter().any(|a| a == "--ia-test") {
        std::process::exit(autoteste_ia(&args));
    }
    if args.iter().any(|a| a == "--uia-test") {
        std::process::exit(autoteste_uia());
    }
    if args.iter().any(|a| a == "--analise-test") {
        std::process::exit(autoteste_analise(&args));
    }

    let config = Config::carregar();
    if config.iniciar_com_sistema {
        autostart::sincronizar(true);
    }
    let estado: Estado = Arc::new(Mutex::new(Compartilhado::novo(config)));
    let (canal, _popups) = motor::iniciar(Arc::clone(&estado));

    let opcoes = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([700.0, 620.0])
            .with_min_inner_size([540.0, 440.0])
            .with_title("Corretor Ortográfico Universal")
            .with_icon(icone_janela()),
        ..Default::default()
    };

    let estado_gui = Arc::clone(&estado);
    let canal_gui = canal.clone();
    eframe::run_native(
        "Corretor Ortográfico Universal",
        opcoes,
        Box::new(move |cc| {
            let (ativo, deteccao, autostart, atalho_config) = estado_gui
                .lock()
                .map(|c| (c.config.ativo, c.config.deteccao_automatica, c.config.iniciar_com_sistema, c.config.atalho.clone()))
                .unwrap_or((true, true, false, config::AtalhoConfig::default()));

            let bandeja = match tray::Bandeja::criar(ativo, deteccao, autostart) {
                Ok(bandeja) => {
                    tray::instalar_handler(canal_gui.clone());
                    Some(bandeja)
                }
                Err(erro) => {
                    motor::logar(&estado_gui, format!("[Aviso] Sem ícone na bandeja: {}", erro));
                    None
                }
            };

            let (hotkey_manager, hotkey_atual, hotkey_id) = criar_atalho(canal_gui.clone(), &atalho_config);
            Ok(Box::new(CorretorApp::new(
                cc,
                Arc::clone(&estado_gui),
                canal_gui.clone(),
                hotkey_manager,
                hotkey_atual,
                hotkey_id,
                bandeja,
            )))
        }),
    )
}

/// `corretor_universal --analise-test "texto" [idioma] [palavra,ignorada]`
/// Mostra a lista de avisos (com as alternativas) que a janela e a seta do
/// "Corrigir" usam.
fn autoteste_analise(args: &[String]) -> i32 {
    let texto = args
        .get(2)
        .cloned()
        .unwrap_or_else(|| "Eu gosto de programaçao em Rust e vc sabe disso.".to_string());
    let idioma = args.get(3).cloned().unwrap_or_else(|| "pt-BR".to_string());
    let dicionario: HashSet<String> = args
        .get(4)
        .map(|s| s.split(',').map(|p| p.trim().to_lowercase()).collect())
        .unwrap_or_default();

    println!("Idioma    : {}", idioma);
    println!("Dicionario: {:?}", dicionario);
    println!("Original  : {}", texto);
    match correcao::analisar(&texto, &idioma, &dicionario) {
        Ok(avisos) => {
            println!("Avisos    : {}", avisos.len());
            for (indice, aviso) in avisos.iter().enumerate() {
                println!(
                    "  {}. [{}] {}\n     regra={} categoria={} mensagem={}",
                    indice + 1,
                    aviso.tipo,
                    aviso.rotulo(),
                    aviso.regra,
                    aviso.categoria,
                    aviso.mensagem
                );
            }
            println!("Aplicáveis: {}", correcao::contar_aplicaveis(&avisos));
            println!("Corrigido : {}", correcao::aplicar_avisos(&texto, &avisos));
            0
        }
        Err(e) => {
            println!("[Erro na API] {}", e);
            1
        }
    }
}

/// Resumo curto para exibir na janela.
fn resumo_gui(texto: &str) -> String {
    let limpo: String = texto.split_whitespace().collect::<Vec<_>>().join(" ");
    if limpo.chars().count() > 60 {
        limpo.chars().take(60).collect::<String>() + "..."
    } else {
        limpo
    }
}

fn icone_janela() -> egui::IconData {
    let (rgba, largura, altura) = tray::pixels_icone();
    egui::IconData { rgba, width: largura, height: altura }
}

/// `corretor_universal --self-test "texto" [idioma] [palavra,ignorada]`
fn autoteste(args: &[String]) -> i32 {
    let texto = args.get(2).cloned().unwrap_or_else(|| "Eu gosto de programaçao em Rust.".to_string());
    let idioma = args.get(3).cloned().unwrap_or_else(|| "pt-BR".to_string());
    let dicionario: HashSet<String> = args
        .get(4)
        .map(|s| s.split(',').map(|p| p.trim().to_lowercase()).collect())
        .unwrap_or_default();

    println!("Idioma    : {}", idioma);
    println!("Dicionario: {:?}", dicionario);
    println!("Original  : {}", texto);
    match correcao::corrigir_via_api(&texto, &idioma, &dicionario) {
        Ok(corrigido) => {
            println!("Corrigido : {}", corrigido);
            0
        }
        Err(e) => {
            println!("[Erro na API] {}", e);
            1
        }
    }
}

/// `corretor_universal --uia-test`: mostra o que o app enxerga no campo com
/// foco. Serve para diagnosticar a detecção automática em um aplicativo.
fn autoteste_uia() -> i32 {
    match deteccao::analisar_foco() {
        Some(foco) => {
            println!("editavel   : {}", foco.editavel);
            println!("tem_selecao: {}", foco.tem_selecao);
            println!("suporta_txt: {}", foco.suporta_texto);
            println!("senha      : {}", foco.senha);
            println!("pid        : {}", foco.pid);
            match deteccao::texto_selecionado() {
                Some(t) => println!("selecao    : {:?}", t),
                None => println!("selecao    : <nenhuma>"),
            }
            0
        }
        None => {
            println!("Nada com foco (ou UI Automation indisponível neste sistema).");
            1
        }
    }
}

/// `corretor_universal --ia-test "texto" [--url U] [--chave K] [--modelo M]`
/// Usa a IA personalizada da configuração, permitindo sobrescrever os campos.
fn autoteste_ia(args: &[String]) -> i32 {
    let texto = args.get(2).cloned().unwrap_or_else(|| "hoje eu fui no mercado".to_string());
    let mut config = Config::carregar();
    let mut indice = 3;
    while indice + 1 < args.len() {
        match args[indice].as_str() {
            "--url" => config.ia_url = args[indice + 1].clone(),
            "--chave" => config.ia_chave = args[indice + 1].clone(),
            "--modelo" => config.ia_modelo = args[indice + 1].clone(),
            "--prompt" => config.ia_prompt = args[indice + 1].clone(),
            outro => {
                println!("Argumento desconhecido: {}", outro);
                return 2;
            }
        }
        indice += 2;
    }

    println!("Endpoint  : {}", ia::endpoint_chat(&config.ia_url));
    println!("Modelo    : {}", config.ia_modelo);
    println!("Original  : {}", texto);
    match ia::melhorar(&texto, &config, ia::Estilo::Padrao) {
        Ok(melhorado) => {
            println!("Melhorado : {}", melhorado);
            0
        }
        Err(e) => {
            println!("[Erro na IA] {}", e);
            1
        }
    }
}

/// Reinicia o aplicativo gerando um novo processo limpo e encerrando o processo atual.
pub fn reiniciar_aplicativo() {
    if let Ok(exe) = std::env::current_exe() {
        let _ = std::process::Command::new(exe).spawn();
        std::process::exit(0);
    }
}

/// Reinicia o aplicativo **como administrador** (ShellExecute com o verbo
/// "runas"). Necessário para que a captura e a colagem funcionem em aplicativos
/// que rodam elevados: o Windows bloqueia o envio de teclado/clipboard entre
/// processos de níveis de integridade diferentes (UIPI).
#[cfg(windows)]
pub fn reiniciar_como_admin() -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe_l: Vec<u16> = exe.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    let operacao: Vec<u16> = "runas".encode_utf16().chain(std::iter::once(0)).collect();

    let resultado = unsafe {
        ShellExecuteW(
            HWND(std::ptr::null_mut()),
            PCWSTR(operacao.as_ptr()),
            PCWSTR(exe_l.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    // HINSTANCE <= 32 indica erro (32 = UAC cancelado pelo usuário).
    if resultado.0 as isize <= 32 {
        return Err("Não foi possível elevar (o pedido de administrador foi cancelado?).".to_string());
    }
    std::process::exit(0);
}

#[cfg(not(windows))]
pub fn reiniciar_como_admin() -> Result<(), String> {
    Err("Reiniciar como administrador só é necessário/suportado no Windows.".to_string())
}
