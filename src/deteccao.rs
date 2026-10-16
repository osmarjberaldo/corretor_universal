//! Detecção automática de texto selecionado.
//!
//! Um thread observa o mouse: quando o usuário termina um arrasto (ou um clique
//! duplo, que seleciona a palavra), consultamos o UI Automation do Windows para
//! saber se o elemento com foco é editável e se tem seleção. Só então o menu
//! aparece — assim ele não aparece em cliques comuns.
//!
//! Fora do Windows não existe detecção automática: o atalho global continua
//! funcionando e o resultado aparece na janela do app.

use std::sync::atomic::{AtomicBool, Ordering};

/// Estado ligado/desligado pela configuração (ativo && deteccao_automatica).
static LIGADO: AtomicBool = AtomicBool::new(false);

pub fn ligar(valor: bool) {
    LIGADO.store(valor, Ordering::Relaxed);
}

/// O que sabemos sobre o campo que está com o foco.
#[derive(Debug, Clone, PartialEq)]
pub struct Foco {
    pub editavel: bool,
    pub tem_selecao: bool,
    pub tem_text_pattern: bool,
    /// O elemento expõe texto para o UI Automation (ValuePattern ou
    /// TextPattern). Muitos controles legados/ somente-leitura não têm
    /// TextPattern, então este é o sinal usado para "dá para selecionar aqui".
    pub suporta_texto: bool,
    pub senha: bool,
    pub pid: u32,
}

pub fn disponivel() -> bool {
    cfg!(windows)
}

// ---------------------------------------------------------------------------
// Windows
// ---------------------------------------------------------------------------
#[cfg(windows)]
mod win {
    use super::{Foco, LIGADO};
    use crate::motor::Mensagem;
    use std::sync::atomic::Ordering;
    use std::sync::mpsc::Sender;
    use std::time::{Duration, Instant};
    use uiautomation::patterns::{UITextPattern, UIValuePattern};
    use uiautomation::UIAutomation;
    use windows::Win32::Foundation::POINT;
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, VK_CONTROL, VK_LBUTTON, VK_MENU,
    };
    use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

    /// Sobe o observador de mouse e teclado, que avisa o motor quando há seleção ou atalho.
    pub fn iniciar(canal: Sender<Mensagem>) {
        std::thread::spawn(move || observar(canal));
    }

    pub fn analisar_foco() -> Option<Foco> {
        let automation = UIAutomation::new().ok()?;
        analisar(&automation)
    }

    /// Texto selecionado no campo com foco, lido pelo UI Automation (não mexe
    /// no clipboard).
    pub fn texto_selecionado() -> Option<String> {
        let automation = UIAutomation::new().ok()?;
        let elemento = automation.get_focused_element().ok()?;
        let texto = elemento.get_pattern::<UITextPattern>().ok()?;
        let faixas = texto.get_selection().ok()?;
        let mut junto = String::new();
        for faixa in faixas {
            if let Ok(parte) = faixa.get_text(-1) {
                junto.push_str(&parte);
            }
        }
        let junto = junto.trim().to_string();
        if junto.is_empty() {
            None
        } else {
            Some(junto)
        }
    }

    /// Conteúdo do campo com foco, quando ele expõe ValuePattern (usado para
    /// conferir se a colagem realmente entrou).
    pub fn valor_focado() -> Option<String> {
        let automation = UIAutomation::new().ok()?;
        let elemento = automation.get_focused_element().ok()?;
        elemento
            .get_pattern::<UIValuePattern>()
            .ok()?
            .get_value()
            .ok()
    }

    fn analisar(automation: &UIAutomation) -> Option<Foco> {
        let elemento = automation.get_focused_element().ok()?;
        let pid = elemento.get_process_id().unwrap_or(0);
        let senha = elemento.is_password().unwrap_or(false);
        let valor = elemento.get_pattern::<UIValuePattern>();
        let editavel = match &valor {
            Ok(padrao) => !padrao.is_readonly().unwrap_or(true),
            Err(_) => false,
        };
        let texto = elemento.get_pattern::<UITextPattern>();
        let tem_text_pattern = texto.is_ok();
        let tem_selecao = match &texto {
            Ok(padrao) => padrao
                .get_selection()
                .map(|faixas| {
                    faixas.iter().any(|faixa| {
                        faixa.get_text(-1).map(|t| !t.trim().is_empty()).unwrap_or(false)
                    })
                })
                .unwrap_or(false),
            Err(_) => false,
        };
        let suporta_texto = valor.is_ok() || texto.is_ok();
        Some(Foco { editavel, tem_selecao, tem_text_pattern, suporta_texto, senha, pid })
    }

    fn cursor() -> (i32, i32) {
        let mut p = POINT::default();
        unsafe {
            let _ = GetCursorPos(&mut p);
        }
        (p.x, p.y)
    }

    fn botao_esquerdo() -> bool {
        unsafe { (GetAsyncKeyState(VK_LBUTTON.0 as i32) & 0x8000u16 as i16) != 0 }
    }

    fn atalho_pressionado() -> bool {
        unsafe {
            let ctrl = (GetAsyncKeyState(VK_CONTROL.0 as i32) & 0x8000u16 as i16) != 0;
            let alt = (GetAsyncKeyState(VK_MENU.0 as i32) & 0x8000u16 as i16) != 0;
            let tecla_c = (GetAsyncKeyState(0x43) & 0x8000u16 as i16) != 0; // 'C' key
            ctrl && alt && tecla_c
        }
    }

    fn observar(canal: Sender<Mensagem>) {
        let automation = UIAutomation::new().ok();
        let proprio_pid = std::process::id();

        let mut antes_pressionado = false;
        let mut antes_hotkey = false;
        let mut inicio = (0, 0);
        let mut instante_pressao = Instant::now();
        let mut ultimo_clique = Instant::now() - Duration::from_secs(10);
        let mut ultima_pos_clique = (0, 0);

        loop {
            // Atalho global Ctrl+Alt+C: verificado de forma nativa e instantânea
            let hotkey_ativo = atalho_pressionado();
            if hotkey_ativo && !antes_hotkey {
                let _ = canal.send(Mensagem::Hotkey);
            }
            antes_hotkey = hotkey_ativo;

            // Se a detecção automática com o mouse estiver desativada, continua verificando apenas o atalho
            if !LIGADO.load(Ordering::Relaxed) {
                antes_pressionado = false;
                std::thread::sleep(Duration::from_millis(35));
                continue;
            }

            let posicao = cursor();
            let pressionado = botao_esquerdo();
            let agora = Instant::now();

            if pressionado && !antes_pressionado {
                inicio = posicao;
                instante_pressao = agora;
            }

            if !pressionado && antes_pressionado {
                let arrasto = (posicao.0 - inicio.0).abs() + (posicao.1 - inicio.1).abs();
                let duracao = agora - instante_pressao;
                let mesma_posicao = (posicao.0 - ultima_pos_clique.0).abs() + (posicao.1 - ultima_pos_clique.1).abs() <= 6;
                let clique_duplo = mesma_posicao
                    && duracao < Duration::from_millis(300)
                    && agora - ultimo_clique < Duration::from_millis(500);
                ultimo_clique = agora;
                ultima_pos_clique = posicao;

                // Processa se houve arrasto real para selecionar texto ou clique duplo
                if arrasto >= 12 || clique_duplo {
                    std::thread::sleep(Duration::from_millis(120));
                    let foco = automation.as_ref().and_then(|a| analisar(a));
                    let pid = foco.as_ref().map(|f| f.pid).unwrap_or(0);
                    let senha = foco.as_ref().map(|f| f.senha).unwrap_or(false);

                    if pid != proprio_pid && !senha {
                        let editavel = foco.as_ref().map(|f| f.editavel).unwrap_or(true);
                        let tem_selecao_real = match &foco {
                            Some(f) => {
                                if f.tem_text_pattern {
                                    f.tem_selecao
                                } else {
                                    arrasto >= 14 || clique_duplo
                                }
                            }
                            None => arrasto >= 14 || clique_duplo,
                        };

                        if tem_selecao_real {
                            let _ = canal.send(Mensagem::Selecao {
                                x: posicao.0,
                                y: posicao.1,
                                editavel,
                            });
                        }
                    }
                }
            }

            antes_pressionado = pressionado;
            std::thread::sleep(Duration::from_millis(35));
        }
    }
}

// ---------------------------------------------------------------------------
// Outros sistemas: sem detecção automática
// ---------------------------------------------------------------------------
#[cfg(not(windows))]
mod outros {
    use super::Foco;
    use crate::motor::Mensagem;
    use std::sync::mpsc::Sender;

    pub fn iniciar(_canal: Sender<Mensagem>) {}

    pub fn analisar_foco() -> Option<Foco> {
        None
    }

    pub fn texto_selecionado() -> Option<String> {
        None
    }

    pub fn valor_focado() -> Option<String> {
        None
    }
}

#[cfg(windows)]
pub use win::{analisar_foco, iniciar, texto_selecionado, valor_focado};

#[cfg(not(windows))]
pub use outros::{analisar_foco, iniciar, texto_selecionado, valor_focado};
