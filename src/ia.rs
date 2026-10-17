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
    requisicao(texto, cfg, estilo.instrucao(cfg).trim(), 0.2, 60)
}

/// Corrige o texto usando a IA (usado como fallback automático quando a API pública oscilar).
pub fn corrigir_com_ia(texto: &str, cfg: &Config) -> Result<String, String> {
    let instrucao = "Você é um corretor ortográfico e gramatical estrito em português do Brasil. \
                     Corrija os erros ortográficos, gramaticais, acentuação e pontuação do texto. \
                     Mantenha rigorosamente o vocabulário e sentido original. \
                     Responda única e exclusivamente com o texto final corrigido: sem comentários, \
                     explicações, títulos, aspas, listas ou perguntas. Não peça mais texto nem \
                     dialogue; se o texto já estiver correto, devolva-o exatamente como está.";
    requisicao(texto, cfg, instrucao, 0.1, 60)
}

/// Ajusta o **contexto/coerência das frases** (não só a ortografia) com a IA.
///
/// Usado quando a opção "Corrigir o contexto da frase com IA" está ativa: além
/// de corrigir a ortografia, a IA revisa concordância, regência e o encaixe das
/// frases no contexto, mantendo o sentido original.
pub fn corrigir_contexto(texto: &str, cfg: &Config) -> Result<String, String> {
    let instrucao = "Você é um revisor de texto em português do Brasil. Além de corrigir ortografia, \
                     gramática, acentuação e pontuação, ajuste o CONTEXTO e a coerência das frases: \
                     corrija concordância, regência, tempos verbais e conexões que não façam sentido \
                     dentro do parágrafo. Preserve o significado, o tom e o vocabulário original; não \
                     acrescente informações novas nem explique nada. \
                     Responda única e exclusivamente com o texto final revisado: sem comentários, \
                     títulos, aspas ou perguntas. Não peça mais texto nem dialogue.";
    requisicao(texto, cfg, instrucao, 0.15, 60)
}

/// Envia uma mensagem de sistema + o texto ao endpoint compatível com OpenAI.
fn requisicao(
    texto: &str,
    cfg: &Config,
    instrucao: &str,
    temperatura: f64,
    timeout_s: u64,
) -> Result<String, String> {
    if !cfg.ia_configurada() {
        return Err("IA personalizada não configurada (URL e modelo são obrigatórios).".to_string());
    }

    let url = endpoint_chat(&cfg.ia_url);
    let corpo = serde_json::json!({
        "model": cfg.ia_modelo.trim(),
        "temperature": temperatura,
        "messages": [
            { "role": "system", "content": instrucao.trim() },
            { "role": "user", "content": texto }
        ]
    });

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(timeout_s))
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
        .map(limpar_resposta)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| format!("resposta sem \"choices[0].message.content\": {}", resumir(&texto_resposta)))
}

/// Remove cercas de código e aspas que alguns modelos adicionam em volta do
/// texto, para o resultado vir "limpo" na prévia.
fn limpar_resposta(bruto: &str) -> String {
    let mut texto = bruto.trim().to_string();
    if texto.starts_with("```") {
        // tira a cerca de abertura (```json) e a de fechamento
        if let Some(quebra) = texto.find('\n') {
            texto = texto[quebra + 1..].to_string();
        }
        if let Some(ultima) = texto.rfind("```") {
            texto.truncate(ultima);
        }
        texto = texto.trim().to_string();
    }
    for par in [("\"", "\""), ("'", "'"), ("“", "”"), ("«", "»")] {
        if texto.starts_with(par.0) && texto.ends_with(par.1) && texto.len() > par.0.len() + par.1.len() {
            texto = texto[par.0.len()..texto.len() - par.1.len()].trim().to_string();
        }
    }
    texto
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

    #[test]
    fn limpa_cercas_e_aspas_da_resposta() {
        assert_eq!(limpar_resposta("Bom dia."), "Bom dia.");
        assert_eq!(limpar_resposta("  \"Bom dia, você.\"  "), "Bom dia, você.");
        assert_eq!(limpar_resposta("```\nBom dia.\n```"), "Bom dia.");
        assert_eq!(limpar_resposta("```json\nBom dia.\n```"), "Bom dia.");
        assert_eq!(limpar_resposta("“Bom dia.”"), "Bom dia.");
    }
}
