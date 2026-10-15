//! Operações de teclado/clipboard: capturar a seleção e substituí-la.

use crate::motor::Estado;
use arboard::Clipboard;
use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use std::thread;
use std::time::Duration;

pub fn ler_clipboard() -> Result<String, String> {
    let mut clipboard = Clipboard::new().map_err(|e| e.to_string())?;
    clipboard.get_text().map_err(|e| e.to_string())
}

pub fn copiar_para_clipboard(texto: &str) -> Result<(), String> {
    let mut clipboard = Clipboard::new().map_err(|e| e.to_string())?;
    clipboard.set_text(texto).map_err(|e| e.to_string())
}

#[cfg(target_os = "macos")]
fn tecla_modificadora() -> Key {
    Key::Meta
}

#[cfg(not(target_os = "macos"))]
fn tecla_modificadora() -> Key {
    Key::Control
}

fn enviar_combinacao(letra: char) -> Result<(), String> {
    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| format!("enigo: {:?}", e))?;
    let modificadora = tecla_modificadora();
    let _ = enigo.key(modificadora, Direction::Press);
    let _ = enigo.key(Key::Unicode(letra), Direction::Click);
    let _ = enigo.key(modificadora, Direction::Release);
    Ok(())
}

/// Copia a seleção do campo com foco (Ctrl+C) e devolve o texto.
pub fn copiar_selecao(_estado: &Estado) -> Result<String, String> {
    // evita devolver o conteúdo anterior do clipboard quando não há seleção
    let anterior = ler_clipboard().unwrap_or_default();
    let _ = copiar_para_clipboard("");

    enviar_combinacao('c')?;
    thread::sleep(Duration::from_millis(220));

    let copiado = ler_clipboard().unwrap_or_default();
    if copiado.trim().is_empty() {
        // devolve o que estava lá antes, para não perder o clipboard do usuário
        let _ = copiar_para_clipboard(&anterior);
        return Ok(String::new());
    }
    Ok(copiado)
}

/// Coloca o texto no clipboard e cola por cima da seleção (Ctrl+V).
/// Devolve `false` quando dá para perceber que o campo não aceitou a colagem.
pub fn substituir_selecao(_estado: &Estado, texto: &str) -> Result<bool, String> {
    let antes = crate::deteccao::valor_focado();

    copiar_para_clipboard(texto)?;
    thread::sleep(Duration::from_millis(150));
    enviar_combinacao('v')?;
    thread::sleep(Duration::from_millis(280));

    match (antes, crate::deteccao::valor_focado()) {
        (Some(a), Some(b)) => Ok(a != b),
        // sem ValuePattern não dá para conferir; considera executado
        _ => Ok(true),
    }
}
