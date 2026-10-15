use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Configuração persistida em disco. Fica em:
/// - Windows: %APPDATA%\corretor_universal\config.json
/// - macOS:   ~/Library/Application Support/corretor_universal/config.json
/// - Linux:   $XDG_CONFIG_HOME/corretor_universal/config.json (ou ~/.config/...)
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Config {
    pub idioma: String,
    pub ativo: bool,
    /// Mostra o menu sozinho quando um texto é selecionado com o mouse.
    pub deteccao_automatica: bool,
    /// Trocou o texto no lugar (Ctrl+V) ou mostrou o balão na última ação.
    pub dicionario: Vec<String>,
    /// Endpoint compatível com OpenAI (".../v1" ou a URL completa de /chat/completions).
    pub ia_url: String,
    pub ia_chave: String,
    pub ia_modelo: String,
    /// Instrução enviada como mensagem de sistema para o "Melhorar texto".
    pub ia_prompt: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            idioma: "pt-BR".to_string(),
            ativo: true,
            deteccao_automatica: true,
            dicionario: Vec::new(),
            ia_url: "https://api.openai.com/v1".to_string(),
            ia_chave: String::new(),
            ia_modelo: "gpt-4o-mini".to_string(),
            ia_prompt: "Reescreva o texto em português do Brasil, corrigindo gramática e \
                        melhorando clareza sem mudar o sentido. Responda apenas com o texto final."
                .to_string(),
        }
    }
}

impl Config {
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
            .join("config.json")
    }

    pub fn carregar() -> Self {
        std::fs::read_to_string(Self::caminho())
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }

    pub fn salvar(&self) -> Result<(), String> {
        let caminho = Self::caminho();
        if let Some(dir) = caminho.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(&caminho, json).map_err(|e| e.to_string())
    }

    /// Só dá para "melhorar" se houver um endpoint e um modelo configurados.
    pub fn ia_configurada(&self) -> bool {
        !self.ia_url.trim().is_empty() && !self.ia_modelo.trim().is_empty()
    }
}
