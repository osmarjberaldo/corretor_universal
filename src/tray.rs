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
const ID_AUTOSTART: &str = "autostart";
const ID_SAIR: &str = "sair";

#[derive(Debug, Clone, PartialEq)]
pub enum AcaoBandeja {
    Abrir,
    AlternarAtivo,
    AlternarDeteccao,
    AlternarAutostart,
    Sair,
}

pub struct Bandeja {
    // Mantém o ícone vivo enquanto o app existir.
    _icone: TrayIcon,
    item_alternar: CheckMenuItem,
    item_deteccao: CheckMenuItem,
    item_autostart: CheckMenuItem,
}

impl Bandeja {
    pub fn criar(ativo: bool, deteccao: bool, autostart: bool) -> Result<Self, String> {
        let menu = Menu::new();
        let item_abrir = MenuItem::with_id(ID_ABRIR, "Abrir janela", true, None);
        let item_alternar = CheckMenuItem::with_id(ID_ALTERNAR, "Captura ativa", true, ativo, None);
        let item_deteccao =
            CheckMenuItem::with_id(ID_DETECCAO, "Menu ao selecionar texto", true, deteccao, None);
        let item_autostart =
            CheckMenuItem::with_id(ID_AUTOSTART, "Iniciar com o sistema", true, autostart, None);
        let item_sair = MenuItem::with_id(ID_SAIR, "Sair", true, None);
        menu.append_items(&[
            &item_abrir,
            &PredefinedMenuItem::separator(),
            &item_alternar,
            &item_deteccao,
            &item_autostart,
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

        Ok(Self {
            _icone: icone,
            item_alternar,
            item_deteccao,
            item_autostart,
        })
    }

    /// Mantém as marcas do menu iguais ao estado do app (que também pode ser
    /// alterado pela própria janela).
    pub fn sincronizar(&self, ativo: bool, deteccao: bool, autostart: bool) {
        if self.item_alternar.is_checked() != ativo {
            self.item_alternar.set_checked(ativo);
        }
        if self.item_deteccao.is_checked() != deteccao {
            self.item_deteccao.set_checked(deteccao);
        }
        if self.item_autostart.is_checked() != autostart {
            self.item_autostart.set_checked(autostart);
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
            ID_AUTOSTART => Some(AcaoBandeja::AlternarAutostart),
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

/// Ícone oficial de alta resolução do aplicativo (idêntico ao .exe),
/// embutido diretamente como 32x32 RGBA para a bandeja e janela.
pub fn pixels_icone() -> (Vec<u8>, u32, u32) {
    const RGBA: &[u8] = include_bytes!("../icone_32.rgba");
    (RGBA.to_vec(), 32, 32)
}
