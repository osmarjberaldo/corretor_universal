// background.js - Service worker da extensão Corretor Universal
// Integração direta com o aplicativo nativo do sistema operacional

const APP_URL = "http://127.0.0.1:23456";

chrome.runtime.onInstalled.addListener(() => {
  criarMenus();
});

chrome.runtime.onStartup.addListener(() => {
  criarMenus();
});

function criarMenus() {
  chrome.contextMenus.removeAll(() => {
    // Menu raiz ao clicar com o botão direito sobre texto selecionado
    chrome.contextMenus.create({
      id: "corretor_root",
      title: "Corretor Universal",
      contexts: ["selection"]
    });

    chrome.contextMenus.create({
      id: "corretor_barra",
      parentId: "corretor_root",
      title: "🪄 Abrir Barra Flutuante do App",
      contexts: ["selection"]
    });

    chrome.contextMenus.create({
      id: "corretor_corrigir",
      parentId: "corretor_root",
      title: "A✓ Corrigir seleção",
      contexts: ["selection"]
    });

    chrome.contextMenus.create({
      id: "corretor_ia_clareza",
      parentId: "corretor_root",
      title: "✨ Melhorar com IA (Clareza)",
      contexts: ["selection"]
    });

    chrome.contextMenus.create({
      id: "corretor_ia_formal",
      parentId: "corretor_root",
      title: "💼 IA: Tornar mais formal",
      contexts: ["selection"]
    });

    chrome.contextMenus.create({
      id: "corretor_ia_simples",
      parentId: "corretor_root",
      title: "💡 IA: Tornar mais simples",
      contexts: ["selection"]
    });

    chrome.contextMenus.create({
      id: "corretor_ia_curto",
      parentId: "corretor_root",
      title: "✂️ IA: Encurtar texto",
      contexts: ["selection"]
    });

    chrome.contextMenus.create({
      id: "corretor_sep",
      parentId: "corretor_root",
      type: "separator",
      contexts: ["selection"]
    });

    chrome.contextMenus.create({
      id: "corretor_balao",
      parentId: "corretor_root",
      title: "💬 Abrir no Balão Nativo do App",
      contexts: ["selection"]
    });

    chrome.contextMenus.create({
      id: "corretor_copiar",
      parentId: "corretor_root",
      title: "❐ Copiar texto corrigido",
      contexts: ["selection"]
    });
  });
}

chrome.contextMenus.onClicked.addListener(async (info, tab) => {
  if (!info.selectionText || !tab || !tab.id) return;

  const texto = info.selectionText.trim();
  const menuId = info.menuItemId;

  let acao = "corrigir";
  switch (menuId) {
    case "corretor_barra":
      acao = "barra";
      break;
    case "corretor_corrigir":
      acao = "corrigir";
      break;
    case "corretor_ia_clareza":
      acao = "ia_clareza";
      break;
    case "corretor_ia_formal":
      acao = "ia_formal";
      break;
    case "corretor_ia_simples":
      acao = "ia_simples";
      break;
    case "corretor_ia_curto":
      acao = "ia_curto";
      break;
    case "corretor_balao":
      acao = "balao";
      break;
    case "corretor_copiar":
      acao = "copiar";
      break;
  }

  // Verifica se o elemento ativo na página é um campo de texto editável
  let isEditable = false;
  try {
    const [execRes] = await chrome.scripting.executeScript({
      target: { tabId: tab.id },
      func: () => {
        const el = document.activeElement;
        return !!(
          el &&
          (el.tagName === "TEXTAREA" ||
            el.tagName === "INPUT" ||
            el.isContentEditable)
        );
      }
    });
    isEditable = !!(execRes && execRes.result);
  } catch (e) {
    isEditable = false;
  }

  // Se o campo não for editável ou for ação de balão/barra, aciona o balão nativo do OS
  const deveMostrarBalao = !isEditable || acao === "balao";

  // Comunica com o aplicativo Corretor Universal no Windows
  try {
    const res = await fetch(`${APP_URL}/executar`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        acao: acao,
        texto: texto,
        mostrar_balao: deveMostrarBalao
      })
    });

    if (res.ok) {
      const data = await res.json();
      if (data.ok && data.resultado) {
        // Se for campo editável e a ação não for apenas barra/balão, substitui o texto no campo
        if (isEditable && acao !== "balao" && acao !== "barra" && acao !== "copiar") {
          chrome.scripting.executeScript({
            target: { tabId: tab.id },
            func: (txt) => {
              try {
                document.execCommand("insertText", false, txt);
              } catch (e) {}
            },
            args: [data.resultado]
          });
        }
        return;
      }
    }
  } catch (e) {
    console.warn("App desktop offline:", e);
  }

  // Se o app desktop estiver fechado, avisa o usuário para abri-lo
  chrome.scripting.executeScript({
    target: { tabId: tab.id },
    func: () => {
      alert(
        "Corretor Universal:\n\nAbra o aplicativo Corretor Universal no seu computador para abrir a barra e o balão nativos do sistema operacional."
      );
    }
  });
});
