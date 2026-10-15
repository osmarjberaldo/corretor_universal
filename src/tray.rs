//! Ícone na área de notificação (bandeja) do sistema, com menu.
//!
//! O app fica ativo com a janela fechada: o ícone mantém tudo vivo e o menu
//! permite reabrir, pausar a captura, ligar/desligar a detecção automática e
//! sair de verdade.

use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

const ID_ABRIR: &str = "abrir";
const ID_ALTERNAR: &str = "alternar";
const ID_DETECCAO: &str = "deteccao";
const ID_SAIR: &str = "sair";

#[derive(Debug, Clone, PartialEq)]
pub enum AcaoBandeja {
    Abrir,
    AlternarAtivo,
    AlternarDeteccao,
    Sair,
}

pub struct Bandeja {
    // Mantém o ícone vivo enquanto o app existir.
    _icone: TrayIcon,
    item_alternar: CheckMenuItem,
    item_deteccao: CheckMenuItem,
}

impl Bandeja {
    pub fn criar(ativo: bool, deteccao: bool) -> Result<Self, String> {
        let menu = Menu::new();
        let item_abrir = MenuItem::with_id(ID_ABRIR, "Abrir janela", true, None);
        let item_alternar = CheckMenuItem::with_id(ID_ALTERNAR, "Captura ativa", true, ativo, None);
        let item_deteccao =
            CheckMenuItem::with_id(ID_DETECCAO, "Menu ao selecionar texto", true, deteccao, None);
        let item_sair = MenuItem::with_id(ID_SAIR, "Sair", true, None);
        menu.append_items(&[
            &item_abrir,
            &PredefinedMenuItem::separator(),
            &item_alternar,
            &item_deteccao,
            &PredefinedMenuItem::separator(),
            &item_sair,
        ])
        .map_err(|e| e.to_string())?;

        let icone = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("Corretor Ortográfico Universal")
            .with_icon(icone_aplicativo())
            .build()
            .map_err(|e| e.to_string())?;

        Ok(Self { _icone: icone, item_alternar, item_deteccao })
    }

    /// Mantém as marcas do menu iguais ao estado do app (que também pode ser
    /// alterado pela própria janela).
    pub fn sincronizar(&self, ativo: bool, deteccao: bool) {
        if self.item_alternar.is_checked() != ativo {
            self.item_alternar.set_checked(ativo);
        }
        if self.item_deteccao.is_checked() != deteccao {
            self.item_deteccao.set_checked(deteccao);
        }
    }
}

/// Encaminha cliques do menu (e clique no ícone) para o motor do app.
pub fn instalar_handler(canal: std::sync::mpsc::Sender<crate::motor::Mensagem>) {
    let para_menu = canal.clone();
    MenuEvent::set_event_handler(Some(move |evento: MenuEvent| {
        let acao = match evento.id.0.as_str() {
            ID_ABRIR => Some(AcaoBandeja::Abrir),
            ID_ALTERNAR => Some(AcaoBandeja::AlternarAtivo),
            ID_DETECCAO => Some(AcaoBandeja::AlternarDeteccao),
            ID_SAIR => Some(AcaoBandeja::Sair),
            _ => None,
        };
        if let Some(acao) = acao {
            let _ = para_menu.send(crate::motor::Mensagem::Bandeja(acao));
        }
    }));

    tray_icon::TrayIconEvent::set_event_handler(Some(move |evento: tray_icon::TrayIconEvent| {
        if let tray_icon::TrayIconEvent::DoubleClick { .. } = evento {
            let _ = canal.send(crate::motor::Mensagem::Bandeja(AcaoBandeja::Abrir));
        }
    }));
}

/// Ícone do app (mesmo desenho da bandeja), usado na janela e no ícone.
pub fn icone_aplicativo() -> Icon {
    let (rgba, largura, altura) = pixels_icone();
    Icon::from_rgba(rgba, largura, altura).expect("ícone válido")
}

/// Ícone desenhado em código (quadrado azul arredondado com um "check"), para
/// não depender de arquivos externos nem da crate `image`.
pub fn pixels_icone() -> (Vec<u8>, u32, u32) {
    const T: usize = 32;
    let mut pixels = vec![0u8; T * T * 4];
    let azul = [32u8, 120, 215, 255];
    let branco = [255u8, 255, 255, 255];

    for y in 0..T {
        for x in 0..T {
            let canto = (x < 6 || x > 25) && (y < 6 || y > 25);
            if x >= 2 && x <= 29 && y >= 2 && y <= 29 && !canto {
                pintar(&mut pixels, T, x, y, azul);
            }
        }
    }
    linha(&mut pixels, T, 9, 17, 14, 23, branco, 1);
    linha(&mut pixels, T, 14, 23, 24, 11, branco, 1);

    (pixels, T as u32, T as u32)
}

fn pintar(pixels: &mut [u8], largura: usize, x: usize, y: usize, cor: [u8; 4]) {
    let i = (y * largura + x) * 4;
    if i + 4 <= pixels.len() {
        pixels[i..i + 4].copy_from_slice(&cor);
    }
}

fn linha(pixels: &mut [u8], largura: usize, x0: i32, y0: i32, x1: i32, y1: i32, cor: [u8; 4], raio: i32) {
    let passos = ((x1 - x0).abs()).max((y1 - y0).abs()).max(1);
    for passo in 0..=passos {
        let x = x0 + (x1 - x0) * passo / passos;
        let y = y0 + (y1 - y0) * passo / passos;
        for dx in -raio..=raio {
            for dy in -raio..=raio {
                let px = x + dx;
                let py = y + dy;
                if px >= 0 && py >= 0 && (px as usize) < largura && (py as usize) < largura {
                    pintar(pixels, largura, px as usize, py as usize, cor);
                }
            }
        }
    }
}
