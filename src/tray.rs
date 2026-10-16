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

#[cfg(windows)]
pub fn mostrar_janela() {
    use windows::core::w;
    use windows::Win32::UI::WindowsAndMessaging::{
        FindWindowW, SetForegroundWindow, ShowWindow, SW_RESTORE, SW_SHOW,
    };
    unsafe {
        if let Ok(hwnd) = FindWindowW(None, w!("Corretor Ortográfico Universal")) {
            if hwnd.0 as usize != 0 {
                let _ = ShowWindow(hwnd, SW_RESTORE);
                let _ = ShowWindow(hwnd, SW_SHOW);
                let _ = SetForegroundWindow(hwnd);
            }
        }
    }
}

#[cfg(not(windows))]
pub fn mostrar_janela() {}

fn despachar_menu(canal: &std::sync::mpsc::Sender<crate::motor::Mensagem>, id: &str) {
    let acao = match id {
        ID_ABRIR => Some(AcaoBandeja::Abrir),
        ID_ALTERNAR => Some(AcaoBandeja::AlternarAtivo),
        ID_DETECCAO => Some(AcaoBandeja::AlternarDeteccao),
        ID_AUTOSTART => Some(AcaoBandeja::AlternarAutostart),
        ID_SAIR => Some(AcaoBandeja::Sair),
        _ => None,
    };
    if let Some(acao) = acao {
        let _ = canal.send(crate::motor::Mensagem::Bandeja(acao));
    }
}

fn despachar_tray(canal: &std::sync::mpsc::Sender<crate::motor::Mensagem>, evento: &tray_icon::TrayIconEvent) {
    match evento {
        tray_icon::TrayIconEvent::DoubleClick { .. } => {
            let _ = canal.send(crate::motor::Mensagem::Bandeja(AcaoBandeja::Abrir));
        }
        tray_icon::TrayIconEvent::Click {
            button: tray_icon::MouseButton::Left,
            button_state: tray_icon::MouseButtonState::Up,
            ..
        } => {
            let _ = canal.send(crate::motor::Mensagem::Bandeja(AcaoBandeja::Abrir));
        }
        _ => {}
    }
}

/// Encaminha cliques do menu (e clique no ícone) para o motor do app.
/// Registra o callback global e também mantém uma thread ouvindo os canais (receiver)
/// para total garantia em qualquer ambiente Windows.
pub fn instalar_handler(canal: std::sync::mpsc::Sender<crate::motor::Mensagem>) {
    let para_menu = canal.clone();
    MenuEvent::set_event_handler(Some(move |evento: MenuEvent| {
        despachar_menu(&para_menu, evento.id.as_ref());
    }));

    let para_tray = canal.clone();
    tray_icon::TrayIconEvent::set_event_handler(Some(move |evento: tray_icon::TrayIconEvent| {
        despachar_tray(&para_tray, &evento);
    }));

    let para_thread = canal;
    std::thread::spawn(move || {
        let menu_rx = MenuEvent::receiver();
        let tray_rx = tray_icon::TrayIconEvent::receiver();
        loop {
            while let Ok(evento) = menu_rx.try_recv() {
                despachar_menu(&para_thread, evento.id.as_ref());
            }
            while let Ok(evento) = tray_rx.try_recv() {
                despachar_tray(&para_thread, &evento);
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    });
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

#[cfg(test)]
mod testes {
    use super::*;
    use std::sync::mpsc::channel;

    #[test]
    fn despacha_todos_os_itens_do_menu() {
        let (tx, rx) = channel();
        despachar_menu(&tx, "abrir");
        despachar_menu(&tx, "alternar");
        despachar_menu(&tx, "deteccao");
        despachar_menu(&tx, "autostart");
        despachar_menu(&tx, "sair");
        despachar_menu(&tx, "desconhecido");

        match rx.recv().unwrap() {
            crate::motor::Mensagem::Bandeja(AcaoBandeja::Abrir) => {}
            outro => panic!("esperado Abrir, obteve {:?}", outro),
        }
        match rx.recv().unwrap() {
            crate::motor::Mensagem::Bandeja(AcaoBandeja::AlternarAtivo) => {}
            outro => panic!("esperado AlternarAtivo, obteve {:?}", outro),
        }
        match rx.recv().unwrap() {
            crate::motor::Mensagem::Bandeja(AcaoBandeja::AlternarDeteccao) => {}
            outro => panic!("esperado AlternarDeteccao, obteve {:?}", outro),
        }
        match rx.recv().unwrap() {
            crate::motor::Mensagem::Bandeja(AcaoBandeja::AlternarAutostart) => {}
            outro => panic!("esperado AlternarAutostart, obteve {:?}", outro),
        }
        match rx.recv().unwrap() {
            crate::motor::Mensagem::Bandeja(AcaoBandeja::Sair) => {}
            outro => panic!("esperado Sair, obteve {:?}", outro),
        }
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn despacha_clique_duplo_e_simples_no_icone() {
        let (tx, rx) = channel();
        let ev_duplo = tray_icon::TrayIconEvent::DoubleClick {
            id: tray_icon::TrayIconId::new("teste"),
            position: tray_icon::dpi::PhysicalPosition::new(0.0, 0.0),
            rect: tray_icon::Rect::default(),
            button: tray_icon::MouseButton::Left,
        };
        despachar_tray(&tx, &ev_duplo);

        match rx.recv().unwrap() {
            crate::motor::Mensagem::Bandeja(AcaoBandeja::Abrir) => {}
            outro => panic!("esperado Abrir para clique duplo, obteve {:?}", outro),
        }

        let ev_simples = tray_icon::TrayIconEvent::Click {
            id: tray_icon::TrayIconId::new("teste"),
            position: tray_icon::dpi::PhysicalPosition::new(0.0, 0.0),
            rect: tray_icon::Rect::default(),
            button: tray_icon::MouseButton::Left,
            button_state: tray_icon::MouseButtonState::Up,
        };
        despachar_tray(&tx, &ev_simples);

        match rx.recv().unwrap() {
            crate::motor::Mensagem::Bandeja(AcaoBandeja::Abrir) => {}
            outro => panic!("esperado Abrir para clique simples, obteve {:?}", outro),
        }
        assert!(rx.try_recv().is_err());
    }
}
