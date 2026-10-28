//! Servidor HTTP local (127.0.0.1:23456) para comunicação bidirecional com a extensão do navegador (Chrome / Edge / Brave).

use crate::motor::{posicao_cursor, Estado, Mensagem};
use crate::popup::Popups;
use std::collections::HashSet;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::Sender;
use std::sync::Arc;

pub const PORTA_PADRAO: u16 = 23456;

#[derive(serde::Deserialize)]
struct RequisicaoExecutar {
    acao: String,
    texto: String,
    #[serde(default)]
    mostrar_balao: bool,
}

pub fn iniciar(estado: Estado, canal: Sender<Mensagem>, popups: Arc<Popups>) {
    std::thread::spawn(move || {
        let listener = match TcpListener::bind(("127.0.0.1", PORTA_PADRAO)) {
            Ok(l) => l,
            Err(e) => {
                crate::motor::logar(
                    &estado,
                    format!("[Servidor Navegador] Porta {} ocupada ou indisponível: {}", PORTA_PADRAO, e),
                );
                return;
            }
        };

        crate::motor::logar(
            &estado,
            format!("[Servidor Navegador] Pronto em http://127.0.0.1:{} para conexão com Chrome/Edge", PORTA_PADRAO),
        );

        for stream in listener.incoming() {
            if let Ok(stream) = stream {
                let estado_clone = Arc::clone(&estado);
                let canal_clone = canal.clone();
                let popups_clone = Arc::clone(&popups);
                std::thread::spawn(move || {
                    tratar_conexao(stream, estado_clone, canal_clone, popups_clone);
                });
            }
        }
    });
}

fn responder(mut stream: TcpStream, status: u16, content_type: &str, corpo: &[u8]) {
    let status_str = match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        500 => "Internal Server Error",
        _ => "OK",
    };
    let cabecalho = format!(
        "HTTP/1.1 {} {}\r\n\
         Access-Control-Allow-Origin: *\r\n\
         Access-Control-Allow-Methods: GET, POST, OPTIONS\r\n\
         Access-Control-Allow-Headers: Content-Type, Authorization\r\n\
         Content-Type: {}\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\r\n",
        status,
        status_str,
        content_type,
        corpo.len()
    );
    let _ = stream.write_all(cabecalho.as_bytes());
    let _ = stream.write_all(corpo);
    let _ = stream.flush();
}

fn responder_json(stream: TcpStream, status: u16, json: &serde_json::Value) {
    let corpo = json.to_string();
    responder(stream, status, "application/json; charset=utf-8", corpo.as_bytes());
}

fn tratar_conexao(
    mut stream: TcpStream,
    estado: Estado,
    _canal: Sender<Mensagem>,
    popups: Arc<Popups>,
) {
    let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(5)));
    let _ = stream.set_write_timeout(Some(std::time::Duration::from_secs(10)));

    let mut buffer = Vec::with_capacity(4096);
    let mut temp = [0u8; 1024];
    let mut cabecalhos_lidos = false;
    let mut content_length: usize = 0;
    let mut header_end_pos: usize = 0;

    // Lê até encontrar \r\n\r\n
    loop {
        match stream.read(&mut temp) {
            Ok(0) => break,
            Ok(n) => {
                buffer.extend_from_slice(&temp[..n]);
                if !cabecalhos_lidos {
                    if let Some(pos) = buffer.windows(4).position(|w| w == b"\r\n\r\n") {
                        cabecalhos_lidos = true;
                        header_end_pos = pos + 4;
                        let cabecalho_str = String::from_utf8_lossy(&buffer[..pos]);
                        for linha in cabecalho_str.lines() {
                            let mut partes = linha.splitn(2, ':');
                            if let (Some(chave), Some(valor)) = (partes.next(), partes.next()) {
                                if chave.trim().eq_ignore_ascii_case("content-length") {
                                    content_length = valor.trim().parse().unwrap_or(0);
                                }
                            }
                        }
                    }
                }
                if cabecalhos_lidos && buffer.len() >= header_end_pos + content_length {
                    break;
                }
            }
            Err(_) => break,
        }
    }

    if !cabecalhos_lidos {
        return;
    }

    let cabecalho_str = String::from_utf8_lossy(&buffer[..header_end_pos]);
    let primeira_linha = cabecalho_str.lines().next().unwrap_or_default();
    let mut partes_requisicao = primeira_linha.split_whitespace();
    let metodo = partes_requisicao.next().unwrap_or("");
    let caminho = partes_requisicao.next().unwrap_or("");

    // Tratamento de CORS Preflight
    if metodo == "OPTIONS" {
        responder(stream, 200, "text/plain", b"");
        return;
    }

    // Endpoint de verificação de status
    if metodo == "GET" && (caminho == "/status" || caminho == "/") {
        let (ativo, ia, idioma) = match estado.lock() {
            Ok(c) => (c.config.ativo, c.config.ia_configurada(), c.config.idioma.clone()),
            Err(_) => (true, false, "pt-BR".to_string()),
        };
        responder_json(
            stream,
            200,
            &serde_json::json!({
                "ok": true,
                "app": "Corretor Universal",
                "versao": "1.1.0",
                "ativo": ativo,
                "ia_configurada": ia,
                "idioma": idioma
            }),
        );
        return;
    }

    // Endpoint de execução de ação (correção ou IA)
    if metodo == "POST" && caminho == "/executar" {
        let corpo_bytes = if buffer.len() >= header_end_pos + content_length {
            &buffer[header_end_pos..header_end_pos + content_length]
        } else {
            &buffer[header_end_pos..]
        };

        let req: RequisicaoExecutar = match serde_json::from_slice(corpo_bytes) {
            Ok(r) => r,
            Err(e) => {
                responder_json(
                    stream,
                    400,
                    &serde_json::json!({
                        "ok": false,
                        "erro": format!("JSON inválido: {}", e)
                    }),
                );
                return;
            }
        };

        let (idioma, dicionario, config) = match estado.lock() {
            Ok(c) => (
                c.config.idioma.clone(),
                c.config.dicionario.iter().cloned().collect::<HashSet<String>>(),
                c.config.clone(),
            ),
            Err(_) => {
                responder_json(
                    stream,
                    500,
                    &serde_json::json!({
                        "ok": false,
                        "erro": "Estado interno do aplicativo indisponível."
                    }),
                );
                return;
            }
        };

        let acao = req.acao.as_str();
        let texto = req.texto;

        let resultado = match acao {
            "corrigir" => {
                crate::motor::logar(&estado, format!("[Extensão] Corrigindo texto: \"{}\"", texto));
                match crate::correcao::corrigir_via_api(&texto, &idioma, &dicionario) {
                    Ok(corrigido) => {
                        if let Ok(mut c) = estado.lock() {
                            c.historico.adicionar(&texto, &corrigido, "LanguageTool (Extensão)");
                            c.resultado = Some(corrigido.clone());
                        }
                        Ok(corrigido)
                    }
                    Err(e) => Err(format!("Falha na correção: {}", e)),
                }
            }
            "ia_clareza" | "ia_formal" | "ia_simples" | "ia_curto" => {
                if !config.ia_configurada() {
                    Err("A IA não está configurada no app. Acesse as Opções do Corretor Universal.".to_string())
                } else {
                    let estilo = match acao {
                        "ia_formal" => crate::ia::Estilo::Formal,
                        "ia_simples" => crate::ia::Estilo::Simples,
                        "ia_curto" => crate::ia::Estilo::Curto,
                        _ => crate::ia::Estilo::Padrao,
                    };
                    let rotulo = match acao {
                        "ia_formal" => "Mais formal",
                        "ia_simples" => "Mais simples",
                        "ia_curto" => "Mais curto",
                        _ => "Clareza e gramática",
                    };
                    crate::motor::logar(&estado, format!("[Extensão] IA ({}): \"{}\"", rotulo, texto));
                    match crate::ia::melhorar(&texto, &config, estilo) {
                        Ok(melhorado) => {
                            if let Ok(mut c) = estado.lock() {
                                c.historico.adicionar(&texto, &melhorado, &format!("IA {} (Extensão)", rotulo));
                                c.resultado = Some(melhorado.clone());
                            }
                            Ok(melhorado)
                        }
                        Err(e) => Err(format!("Falha na IA: {}", e)),
                    }
                }
            }
            "copiar" => {
                let _ = crate::aplicacao::copiar_para_clipboard(&texto);
                Ok(texto)
            }
            "balao" => {
                let pos = posicao_cursor();
                popups.balao(pos.0, pos.1, texto.clone());
                Ok(texto)
            }
            "barra" => {
                let pos = posicao_cursor();
                let itens = match estado.lock() {
                    Ok(c) => crate::motor::itens_menu(&c.analise, c.analisando, c.config.ia_configurada()),
                    Err(_) => Vec::new(),
                };
                popups.barra(pos.0, pos.1, itens);
                Ok(texto)
            }
            outro => Err(format!("Ação desconhecida: {}", outro)),
        };

        match resultado {
            Ok(saida) => {
                if let Ok(mut c) = estado.lock() {
                    c.resultado = Some(saida.clone());
                }
                if req.mostrar_balao {
                    let pos = posicao_cursor();
                    popups.balao(pos.0, pos.1, saida.clone());
                }
                responder_json(
                    stream,
                    200,
                    &serde_json::json!({
                        "ok": true,
                        "resultado": saida
                    }),
                );
            }
            Err(erro) => {
                responder_json(
                    stream,
                    500,
                    &serde_json::json!({
                        "ok": false,
                        "erro": erro
                    }),
                );
            }
        }
        return;
    }

    responder(stream, 404, "text/plain", b"404 Not Found");
}
