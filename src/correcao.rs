//! Verificação pelo LanguageTool (API pública).
//!
//! Em vez de aplicar `replacements[0]` de tudo (o que "corrigia" nomes próprios
//! e termos técnicos), o módulo devolve a **lista de avisos** com todas as
//! alternativas que a própria API ofereceu — cada um pode ser aceito, ter outra
//! sugestão escolhida ou ser ignorado, exatamente como no painel do
//! LanguageTool.
//!
//! Documentação da API (limites da pública: 20 KB por requisição, 20 req/min):
//! <https://dev.languagetool.org/public-http-api.html>

use serde::Deserialize;
use std::collections::HashSet;
use std::time::Duration;

const ENDPOINT: &str = "https://api.languagetool.org/v2/check";
const TIMEOUT: Duration = Duration::from_secs(6);

#[derive(Deserialize, Debug)]
struct LtResponse {
    matches: Vec<LtMatch>,
}

#[derive(Deserialize, Debug)]
struct LtMatch {
    message: String,
    offset: usize,
    length: usize,
    #[serde(default)]
    replacements: Vec<LtReplacement>,
    rule: LtRule,
}

#[derive(Deserialize, Debug)]
struct LtRule {
    id: String,
    #[serde(rename = "issueType", default)]
    issue_type: Option<String>,
    category: LtCategory,
}

#[derive(Deserialize, Debug)]
struct LtCategory {
    id: String,
}

#[derive(Deserialize, Debug)]
struct LtReplacement {
    value: String,
}

/// Um aviso da verificação, com as alternativas oferecidas pela API.
#[derive(Debug, Clone, PartialEq)]
pub struct Aviso {
    /// Posição (em caracteres) do trecho no texto analisado.
    pub offset: usize,
    pub length: usize,
    /// O trecho que o LanguageTool apontou.
    pub trecho: String,
    /// Explicação do aviso ("Possível erro de ortografia.", "Esta locução...").
    pub mensagem: String,
    /// Id da regra (`MORFOLOGIK_RULE_PT_BR`, `PT_WORDINESS_...`).
    pub regra: String,
    /// Categoria da regra (`TYPOS`, `REDUNDANCY`, `FORMAL`, ...).
    pub categoria: String,
    /// Tipo do problema (`misspelling`, `style`, `grammar`, ...).
    pub tipo: String,
    /// Alternativas oferecidas (a primeira é a que a API considera melhor).
    pub sugestoes: Vec<String>,
    /// Alternativa escolhida para este aviso (índice em `sugestoes`).
    pub escolhida: usize,
    /// Vai ser aplicado? (desmarcar deixa o restante do texto em paz)
    pub aceito: bool,
    /// Palavra do dicionário pessoal: aparece na lista, mas nunca é trocada.
    pub protegido: bool,
}

impl Aviso {
    /// Sugestão que seria aplicada agora.
    pub fn sugestao(&self) -> Option<&str> {
        self.sugestoes.get(self.escolhida).map(|s| s.as_str())
    }

    /// Este aviso vai mudar o texto?
    pub fn sera_aplicado(&self) -> bool {
        self.aceito && !self.protegido && self.sugestao().is_some()
    }

    /// Rótulo curto para a lista e para o flyout da seta.
    pub fn rotulo(&self) -> String {
        let mut rotulo = format!("{} → {}", self.trecho, self.sugestao().unwrap_or("(sem sugestão)"));
        if !self.tipo.is_empty() {
            rotulo.push_str(&format!("  [{}]", self.tipo));
        }
        if self.protegido {
            rotulo.push_str("  (dicionário: não trocar)");
        }
        rotulo
    }
}

/// Verifica o texto e devolve os avisos, na ordem em que aparecem no texto.
pub fn analisar(
    texto: &str,
    idioma: &str,
    dicionario: &HashSet<String>,
) -> Result<Vec<Aviso>, String> {
    let client = reqwest::blocking::Client::new();
    let params = [("text", texto), ("language", idioma)];

    let res = client
        .post(ENDPOINT)
        .form(&params)
        .timeout(TIMEOUT)
        .send()
        .map_err(|e| e.to_string())?;

    if !res.status().is_success() {
        return Err(format!("HTTP {}", res.status()));
    }

    let dados: LtResponse = res.json().map_err(|e| e.to_string())?;
    montar_avisos(texto, dados.matches, dicionario)
}

/// Monta a lista de avisos (separado da rede para poder ser testado).
fn montar_avisos(
    texto: &str,
    matches: Vec<LtMatch>,
    dicionario: &HashSet<String>,
) -> Result<Vec<Aviso>, String> {
    let chars: Vec<char> = texto.chars().collect();
    let mut avisos = Vec::with_capacity(matches.len());

    for m in matches {
        if m.offset + m.length > chars.len() {
            continue;
        }
        let trecho: String = chars[m.offset..m.offset + m.length].iter().collect();
        let protegido = dicionario.contains(&trecho.to_lowercase());
        avisos.push(Aviso {
            offset: m.offset,
            length: m.length,
            trecho,
            mensagem: m.message,
            regra: m.rule.id,
            categoria: m.rule.category.id,
            tipo: m.rule.issue_type.unwrap_or_default(),
            sugestoes: m.replacements.into_iter().map(|r| r.value).collect(),
            escolhida: 0,
            aceito: true,
            protegido,
        });
    }

    avisos.sort_by_key(|a| a.offset);
    Ok(avisos)
}

/// Aplica os avisos marcados, de trás para a frente (para os índices de
/// caracteres continuarem válidos).
pub fn aplicar_avisos(texto: &str, avisos: &[Aviso]) -> String {
    let mut chars: Vec<char> = texto.chars().collect();
    let mut marcados: Vec<&Aviso> = avisos.iter().filter(|a| a.sera_aplicado()).collect();
    marcados.sort_by(|a, b| b.offset.cmp(&a.offset));

    for aviso in marcados {
        if aviso.offset + aviso.length > chars.len() {
            continue;
        }
        if let Some(sugestao) = aviso.sugestao() {
            chars.splice(aviso.offset..aviso.offset + aviso.length, sugestao.chars());
        }
    }

    chars.into_iter().collect()
}

/// Quantos avisos vão mudar o texto.
pub fn contar_aplicaveis(avisos: &[Aviso]) -> usize {
    avisos.iter().filter(|a| a.sera_aplicado()).count()
}

/// Verifica e aplica todos os avisos (mantém o comportamento antigo, usado nos
/// modos de linha de comando).
pub fn corrigir_via_api(
    texto: &str,
    idioma: &str,
    dicionario: &HashSet<String>,
) -> Result<String, String> {
    let avisos = analisar(texto, idioma, dicionario)?;
    Ok(aplicar_avisos(texto, &avisos))
}

#[cfg(test)]
mod testes {
    use super::*;

    fn aviso(trecho: &str, offset: usize, sugestoes: &[&str]) -> Aviso {
        Aviso {
            offset,
            length: trecho.chars().count(),
            trecho: trecho.to_string(),
            mensagem: String::new(),
            regra: "REGRA".to_string(),
            categoria: "TYPOS".to_string(),
            tipo: "misspelling".to_string(),
            sugestoes: sugestoes.iter().map(|s| s.to_string()).collect(),
            escolhida: 0,
            aceito: true,
            protegido: false,
        }
    }

    #[test]
    fn dicionario_protege_a_palavra() {
        let mut dicionario = HashSet::new();
        dicionario.insert("rust".to_string());
        assert!(dicionario.contains(&"Rust".to_lowercase()));
    }

    #[test]
    fn aplica_somente_os_avisos_marcados() {
        let texto = "Eu gosto de programaçao em Rust.";
        //          0123456789...
        let programaçao = aviso("programaçao", 12, &["programação"]);
        let mut rust = aviso("Rust", 27, &["Rush"]);
        rust.aceito = false;
        // o aviso desmarcado é clonado como imutável no vetor final
        let avisos = vec![programaçao.clone(), rust.clone()];
        // confere que o desmarcado não entra
        assert_eq!(contar_aplicaveis(&avisos), 1);
        let saida = aplicar_avisos(texto, &avisos);
        assert_eq!(saida, "Eu gosto de programação em Rust.");

        // e que marcar o segundo aplica a sugestão dele
        rust.aceito = true;
        let avisos = vec![programaçao, rust];
        assert_eq!(aplicar_avisos(texto, &avisos), "Eu gosto de programação em Rush.");
    }

    #[test]
    fn escolhe_a_alternativa_pedida() {
        let texto = "Nos dias de hoje chove.";
        let mut dias = aviso("Nos dias de hoje", 0, &["Atualmente", "Presentemente"]);
        dias.escolhida = 1;
        assert_eq!(aplicar_avisos(texto, &[dias]), "Presentemente chove.");
    }

    #[test]
    fn palavra_protegida_nao_e_trocada() {
        let texto = "Eu gosto de Rust.";
        let mut rust = aviso("Rust", 12, &["Rush"]);
        rust.protegido = true;
        assert!(!rust.sera_aplicado());
        assert_eq!(aplicar_avisos(texto, &[rust]), texto);
    }

    #[test]
    fn avisos_desordenados_sao_aplicados_de_tras_para_frente() {
        let texto = "programaçao e Rust";
        let a = aviso("programaçao", 0, &["programação"]);
        let b = aviso("Rust", 14, &["Rust!"]);
        assert_eq!(aplicar_avisos(texto, &[b, a]), "programação e Rust!");
    }

    #[test]
    fn montar_avisos_marca_o_dicionario() {
        let dicionario: HashSet<String> = ["rust".to_string()].into_iter().collect();
        let matches = vec![LtMatch {
            message: "Possível erro de ortografia.".to_string(),
            offset: 12,
            length: 4,
            replacements: vec![LtReplacement { value: "Rush".to_string() }],
            rule: LtRule {
                id: "MORFOLOGIK_RULE_PT_BR".to_string(),
                issue_type: Some("misspelling".to_string()),
                category: LtCategory { id: "TYPOS".to_string() },
            },
        }];
        let avisos = montar_avisos("Eu gosto de Rust.", matches, &dicionario).unwrap();
        assert_eq!(avisos.len(), 1);
        assert!(avisos[0].protegido);
        assert_eq!(avisos[0].trecho, "Rust");
        assert_eq!(avisos[0].tipo, "misspelling");
        assert_eq!(avisos[0].rotulo(), "Rust → Rush  [misspelling]  (dicionário: não trocar)");
    }

    #[test]
    fn aviso_fora_do_texto_e_descartado() {
        let matches = vec![LtMatch {
            message: String::new(),
            offset: 999,
            length: 5,
            replacements: Vec::new(),
            rule: LtRule {
                id: "X".to_string(),
                issue_type: None,
                category: LtCategory { id: "TYPOS".to_string() },
            },
        }];
        assert!(montar_avisos("curto", matches, &HashSet::new()).unwrap().is_empty());
    }
}
