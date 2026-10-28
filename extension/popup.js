// popup.js - Checa status do aplicativo desktop local
const APP_URL = "http://127.0.0.1:23456";

document.addEventListener("DOMContentLoaded", async () => {
  const elStatusApp = document.getElementById("status-app");
  const elStatusIa = document.getElementById("status-ia");

  try {
    const res = await fetch(`${APP_URL}/status`);
    if (res.ok) {
      const data = await res.json();
      elStatusApp.className = "badge badge-online";
      elStatusApp.textContent = "Conectado";

      if (data.ia_configurada) {
        elStatusIa.textContent = "✨ Ativa";
        elStatusIa.style.color = "#4ade80";
      } else {
        elStatusIa.textContent = "Configurar no app";
        elStatusIa.style.color = "#facc15";
      }
      return;
    }
  } catch (e) {
    // App não está rodando
  }

  elStatusApp.className = "badge badge-offline";
  elStatusApp.textContent = "Desconectado";
  elStatusIa.textContent = "LanguageTool direto";
  elStatusIa.style.color = "#a1a1aa";
});
