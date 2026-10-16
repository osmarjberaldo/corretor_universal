use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Limite máximo de itens no histórico (exatamente 10 mais recentes).
pub const MAX_HISTORICO: usize = 10;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ItemHistorico {
    pub original: String,
    pub corrigido: String,
    pub tipo: String,
    pub data_hora: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Historico {
    pub itens: Vec<ItemHistorico>,
}

impl Historico {
    pub fn caminho() -> PathBuf {
        let base = if cfg!(target_os = "windows") {
            std::env::var_os("APPDATA").map(PathBuf::from)
        } else if cfg!(target_os = "macos") {
            std::env::var_os("HOME")
                .map(|h| PathBuf::from(h).join("Library").join("Application Support"))
        } else {
            std::env::var_os("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        };
        base.unwrap_or_else(|| PathBuf::from("."))
            .join("corretor_universal")
            .join("historico.json")
    }

    pub fn carregar() -> Self {
        let mut h: Self = std::fs::read_to_string(Self::caminho())
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default();
        if h.itens.len() > MAX_HISTORICO {
            h.itens.truncate(MAX_HISTORICO);
        }
        h
    }

    pub fn salvar(&self) -> Result<(), String> {
        let caminho = Self::caminho();
        if let Some(dir) = caminho.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(&caminho, json).map_err(|e| e.to_string())
    }

    pub fn adicionar(&mut self, original: &str, corrigido: &str, tipo: &str) {
        let orig = original.trim();
        let corr = corrigido.trim();
        if orig.is_empty() || orig == corr {
            return;
        }

        // Remove duplicata prévia idêntica para manter o mais recente no topo
        self.itens.retain(|i| !(i.original == orig && i.corrigido == corr));

        let novo = ItemHistorico {
            original: orig.to_string(),
            corrigido: corr.to_string(),
            tipo: tipo.to_string(),
            data_hora: data_hora_atual(),
        };

        self.itens.insert(0, novo);
        if self.itens.len() > MAX_HISTORICO {
            self.itens.truncate(MAX_HISTORICO);
        }
        let _ = self.salvar();
    }

    pub fn limpar(&mut self) {
        self.itens.clear();
        let _ = self.salvar();
    }
}

#[cfg(windows)]
fn data_hora_atual() -> String {
    #[repr(C)]
    #[allow(non_snake_case)]
    struct SYSTEMTIME {
        wYear: u16,
        wMonth: u16,
        wDayOfWeek: u16,
        wDay: u16,
        wHour: u16,
        wMinute: u16,
        wSecond: u16,
        wMilliseconds: u16,
    }
    extern "system" {
        fn GetLocalTime(lpSystemTime: *mut SYSTEMTIME);
    }
    unsafe {
        let mut st = std::mem::zeroed();
        GetLocalTime(&mut st);
        format!(
            "{:02}:{:02}:{:02} - {:02}/{:02}",
            st.wHour, st.wMinute, st.wSecond, st.wDay, st.wMonth
        )
    }
}

#[cfg(not(windows))]
fn data_hora_atual() -> String {
    let now = std::time::SystemTime::now();
    let dur = now.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    let secs = dur.as_secs();
    let h = (secs / 3600) % 24;
    let m = (secs / 60) % 60;
    let s = secs % 60;
    format!("{:02}:{:02}:{:02}", h, m, s)
}
