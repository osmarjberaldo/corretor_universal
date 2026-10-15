#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // Oculta o console no Windows em modo release

mod aplicacao;
mod config;
mod correcao;
mod deteccao;
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
use std::sync::{Arc, Mutex};
use std::time::Duration;

struct CorretorApp {
    estado: Estado,
    canal: std::sync::mpsc::Sender<Mensagem>,
    _hotkey_manager: GlobalHotKeyManager,
    _bandeja: Option<tray::Bandeja>,
    nova_palavra: String,
    saindo: bool,
}

impl CorretorApp {
    fn new(
        _cc: &eframe::CreationContext<'_>,
        estado: Estado,
        canal: std::sync::mpsc::Sender<Mensagem>,
        hotkey_manager: GlobalHotKeyManager,
        bandeja: Option<tray::Bandeja>,
    ) -> Self {
        Self {
            estado,
            canal,
            _hotkey_manager: hotkey_manager,
            _bandeja: bandeja,
            nova_palavra: String::new(),
            saindo: false,
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
        // Fechar a janela manda para a bandeja (o app continua ativo).
        let pedido_fechar = ctx.input(|i| i.viewport().close_requested());
        if pedido_fechar && !self.saindo {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }

        // Pedido de reabertura vindo da bandeja/atalho.
        let pedido = self.estado.lock().ok().and_then(|mut c| c.janela.take());
        if pedido == Some(true) {
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

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Corretor Ortográfico Universal");
            ui.add_space(4.0);

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
                    egui::Checkbox::new(&mut est.config.deteccao_automatica, "Menu ao selecionar texto"),
                )
                .on_hover_text(if deteccao::disponivel() {
                    "Abre o menu sozinho quando você seleciona um texto com o mouse"
                } else {
                    "Detecção automática só existe no Windows; use o atalho global"
                });
                if est.config.deteccao_automatica != est.deteccao_anterior {
                    est.deteccao_anterior = est.config.deteccao_automatica;
                    let _ = est.config.salvar();
                }
                let atalho = if cfg!(target_os = "macos") { "Cmd + Alt + C" } else { "Ctrl + Alt + C" };
                ui.label(format!("| atalho: {}", atalho));
            });

            ui.separator();

            ui.horizontal(|ui| {
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
                ui.label(if popup::Popups::suportado() {
                    "| menu e balão nativos"
                } else {
                    "| popups nativos indisponíveis neste sistema"
                });
            });

            ui.add_space(4.0);
            ui.group(|ui| {
                ui.label(egui::RichText::new("Dicionário pessoal (palavras que não devem ser corrigidas):").strong());
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
                    ui.label(format!("({} ignoradas)", est.config.dicionario.len()));
                });
            });

            ui.add_space(4.0);
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
                ui.horizontal(|ui| {
                    if ui.button("Salvar IA").clicked() {
                        match est.config.salvar() {
                            Ok(_) => {
                                let caminho = Config::caminho();
                                est.status_ia = format!("Configuração salva em {}", caminho.display());
                            }
                            Err(e) => est.status_ia = format!("Erro ao salvar: {}", e),
                        }
                    }
                    if ui.button("Testar IA").clicked() {
                        let config = est.config.clone();
                        let estado = Arc::clone(&self.estado);
                        est.status_ia = "Testando...".to_string();
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

            ui.add_space(4.0);
            painel_avisos(ui, &mut est, &canal);

            ui.add_space(4.0);
            ui.label(egui::RichText::new("Último resultado:").strong());
            ui.add(
                egui::TextEdit::multiline(&mut est.resultado.clone().unwrap_or_default())
                    .desired_rows(2)
                    .desired_width(f32::INFINITY)
                    .interactive(false),
            );

            ui.add_space(4.0);
            ui.label(egui::RichText::new("Histórico:").strong());
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    for log in &est.logs {
                        ui.monospace(log);
                    }
                });
        });

        // Mantém o estado da bandeja e da detecção iguais à configuração.
        if let Some(bandeja) = &self._bandeja {
            bandeja.sincronizar(est.config.ativo, est.config.deteccao_automatica);
        }
        deteccao::ligar(est.config.ativo && est.config.deteccao_automatica);
        drop(est);

        ctx.request_repaint_after(Duration::from_millis(150));
    }
}

fn modificador_atalho() -> Modifiers {
    #[cfg(target_os = "macos")]
    {
        Modifiers::META | Modifiers::ALT
    }
    #[cfg(not(target_os = "macos"))]
    {
        Modifiers::CONTROL | Modifiers::ALT
    }
}

fn criar_atalho(canal: std::sync::mpsc::Sender<Mensagem>) -> GlobalHotKeyManager {
    let manager = GlobalHotKeyManager::new().expect("Falha ao inicializar GlobalHotKeyManager");
    let hotkey = HotKey::new(Some(modificador_atalho()), Code::KeyC);
    manager.register(hotkey).expect("Falha ao registrar o atalho global");
    let id = hotkey.id();
    GlobalHotKeyEvent::set_event_handler(Some(move |evento: GlobalHotKeyEvent| {
        if evento.id == id && evento.state == HotKeyState::Pressed {
            let _ = canal.send(Mensagem::Hotkey);
        }
    }));
    manager
}

fn main() -> eframe::Result<()> {
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
    let estado: Estado = Arc::new(Mutex::new(Compartilhado::novo(config)));
    let (canal, _popups) = motor::iniciar(Arc::clone(&estado));

    let opcoes = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([680.0, 560.0])
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
            let (ativo, deteccao) = estado_gui
                .lock()
                .map(|c| (c.config.ativo, c.config.deteccao_automatica))
                .unwrap_or((true, true));

            let bandeja = match tray::Bandeja::criar(ativo, deteccao) {
                Ok(bandeja) => {
                    tray::instalar_handler(canal_gui.clone());
                    Some(bandeja)
                }
                Err(erro) => {
                    motor::logar(&estado_gui, format!("[Aviso] Sem ícone na bandeja: {}", erro));
                    None
                }
            };

            let hotkey_manager = criar_atalho(canal_gui.clone());
            Ok(Box::new(CorretorApp::new(
                cc,
                Arc::clone(&estado_gui),
                canal_gui.clone(),
                hotkey_manager,
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
