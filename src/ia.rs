use crate::config::Config;
use std::time::Duration;

/// Aceita tanto a base ("https://api.openai.com/v1", ".../v1/") quanto a URL
/// completa do endpoint de chat.
pub fn endpoint_chat(url: &str) -> String {
    let base = url.trim().trim_end_matches('/');
    if base.ends_with("/chat/completions") {
        base.to_string()
    } else {
        format!("{}/chat/completions", base)
    }
}

/// Estilo de reescrita pedido pelo botão "Melhorar" (variantes do flyout).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Estilo {
    /// Usa o prompt personalizado da configuração.
    Padrao,
    Formal,
    Simples,
    Curto,
}

impl Estilo {
    /// Instrução (mensagem de sistema) enviada ao modelo.
    pub fn instrucao(self, cfg: &Config) -> String {
        match self {
            Estilo::Padrao => cfg.ia_prompt.clone(),
            Estilo::Formal => "Reescreva o texto em português do Brasil com tom formal e profissional, corrigindo a gramática e mantendo o sentido. Responda apenas com o texto final.".to_string(),
            Estilo::Simples => "Reescreva o texto em português do Brasil com linguagem simples e direta, corrigindo a gramática e mantendo o sentido. Responda apenas com o texto final.".to_string(),
            Estilo::Curto => "Reescreva o texto em português do Brasil de forma mais curta e objetiva, sem perder o sentido. Responda apenas com o texto final.".to_string(),
        }
    }
}

/// Reescreve o texto usando o endpoint configurado em "IA personalizada".
pub fn melhorar(texto: &str, cfg: &Config, estilo: Estilo) -> Result<String, String> {
    if !cfg.ia_configurada() {
        return Err("IA personalizada não configurada (URL e modelo são obrigatórios).".to_string());
    }

    let url = endpoint_chat(&cfg.ia_url);
    let corpo = serde_json::json!({
        "model": cfg.ia_modelo.trim(),
        "temperature": 0.2,
        "messages": [
            { "role": "system", "content": estilo.instrucao(cfg).trim() },
            { "role": "user", "content": texto }
        ]
    });

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;

    let mut req = client.post(&url).json(&corpo);
    if !cfg.ia_chave.trim().is_empty() {
        req = req.bearer_auth(cfg.ia_chave.trim());
    }

    let res = req.send().map_err(|e| format!("{} ({})", e, url))?;
    let status = res.status();
    let texto_resposta = res.text().map_err(|e| e.to_string())?;

    if !status.is_success() {
        return Err(format!("HTTP {}: {}", status, resumir(&texto_resposta)));
    }

    let json: serde_json::Value =
        serde_json::from_str(&texto_resposta).map_err(|e| format!("JSON inválido: {}", e))?;

    json["choices"][0]["message"]["content"]
        .as_str()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| format!("resposta sem \"choices[0].message.content\": {}", resumir(&texto_resposta)))
}

fn resumir(texto: &str) -> String {
    let limpo: String = texto.split_whitespace().collect::<Vec<_>>().join(" ");
    if limpo.chars().count() > 300 {
        let corte: String = limpo.chars().take(300).collect();
        format!("{}...", corte)
    } else {
        limpo
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn completa_a_url_do_endpoint() {
        assert_eq!(endpoint_chat("https://api.openai.com/v1"), "https://api.openai.com/v1/chat/completions");
        assert_eq!(endpoint_chat("https://api.openai.com/v1/"), "https://api.openai.com/v1/chat/completions");
        assert_eq!(
            endpoint_chat("http://127.0.0.1:8080/v1/chat/completions"),
            "http://127.0.0.1:8080/v1/chat/completions"
        );
    }
}
