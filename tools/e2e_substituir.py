"""Teste do botão "Substituir" do balão de resultado.

Caso real (WordCraft): há apps cujo campo é editável e aceita Ctrl+V, mas que
não são detectados como editáveis (ex.: expõem o campo como "Document" com
ValuePattern somente-leitura, ou não expõem nada). Antes o corretor só copiava;
agora o balão de resultado traz o botão "Substituir", que cola o texto no campo.

Usa uma janela Tkinter (Text) — que aceita copiar/colar mas não é reconhecida
pela detecção — para provar de ponta a ponta:
1. a barra abre pelo atalho global (Ctrl+Alt+C);
2. o balão de resultado traz "Substituir", "Copiar resultado" e
   "Gerar novo texto com IA";
3. clicar em "Substituir" realmente cola o texto corrigido no campo.

Uso: python tools/e2e_substituir.py [--exe CAMINHO]
"""
import ctypes
import subprocess
import sys
import time
import tkinter as tk
from ctypes import wintypes

sys.stdout.reconfigure(encoding="utf-8", errors="replace")

user32 = ctypes.windll.user32
kernel32 = ctypes.windll.kernel32

user32.FindWindowW.restype = wintypes.HWND
user32.FindWindowW.argtypes = [wintypes.LPCWSTR, wintypes.LPCWSTR]
user32.IsWindowVisible.argtypes = [wintypes.HWND]
user32.GetWindowTextW.restype = ctypes.c_int
user32.SetForegroundWindow.argtypes = [wintypes.HWND]
user32.FindWindowW.restype = wintypes.HWND

CLASSE_BARRA = "CorretorMenuPopup"
CLASSE_BALAO = "CorretorBalaoPopup"

TEXTO = "Eu gosto de programaçao em Pyton ."


class KEYBDINPUT(ctypes.Structure):
    _fields_ = [("wVk", wintypes.WORD), ("wScan", wintypes.WORD), ("dwFlags", wintypes.DWORD),
                ("time", wintypes.DWORD), ("dwExtraInfo", ctypes.c_void_p)]


class _ENTRADA(ctypes.Union):
    _fields_ = [("ki", KEYBDINPUT), ("padding", ctypes.c_byte * 32)]


class INPUT(ctypes.Structure):
    _fields_ = [("type", wintypes.DWORD), ("u", _ENTRADA)]


user32.SendInput.restype = wintypes.UINT
user32.SendInput.argtypes = [wintypes.UINT, ctypes.POINTER(INPUT), ctypes.c_int]


def tecla(vk, soltar=False, pausa=0.06):
    entrada = INPUT()
    entrada.type = 1
    entrada.u.ki = KEYBDINPUT(wVk=vk, wScan=0, dwFlags=2 if soltar else 0, time=0, dwExtraInfo=None)
    user32.SendInput(1, ctypes.byref(entrada), ctypes.sizeof(INPUT))
    time.sleep(pausa)


def atalho_global():
    """Ctrl+Alt+C segurado por tempo suficiente para a detecção por polling pegar."""
    tecla(0x11, pausa=0.08)
    tecla(0x12, pausa=0.08)
    tecla(0x43, pausa=0.20)
    tecla(0x43, True, pausa=0.06)
    tecla(0x12, True, pausa=0.06)
    tecla(0x11, True, pausa=0.06)


def esc():
    tecla(0x1B, pausa=0.05)
    tecla(0x1B, True, pausa=0.05)


def clicar(x, y):
    user32.SetCursorPos(x, y)
    time.sleep(0.4)
    user32.mouse_event(0x0002, 0, 0, 0, 0)
    time.sleep(0.15)
    user32.mouse_event(0x0004, 0, 0, 0, 0)
    time.sleep(0.8)


def texto_janela(hwnd):
    n = user32.GetWindowTextLengthW(hwnd)
    if n <= 0:
        return ""
    buf = ctypes.create_unicode_buffer(n + 1)
    user32.GetWindowTextW(hwnd, buf, n + 1)
    return buf.value


def filhos(hwnd):
    achados = []

    @ctypes.WINFUNCTYPE(wintypes.BOOL, wintypes.HWND, wintypes.LPARAM)
    def cb(h, _):
        achados.append(h)
        return True

    user32.EnumChildWindows(hwnd, cb, 0)
    return achados


def rect(hwnd):
    r = wintypes.RECT()
    user32.GetWindowRect(hwnd, ctypes.byref(r))
    return r


def botao(janela, prefixo):
    for f in filhos(janela):
        t = texto_janela(f)
        if t and t.lower().startswith(prefixo.lower()):
            return f, t
    return None, None


def visivel(classe):
    h = user32.FindWindowW(classe, None)
    return bool(h) and bool(user32.IsWindowVisible(h))


def app_rodando():
    out = subprocess.run(["tasklist", "/FI", "IMAGENAME eq corretor_universal.exe", "/FO", "CSV", "/NH"],
                         capture_output=True, text=True).stdout
    return "corretor_universal.exe" in out


resultado = {"erros": [], "textos_balao": None, "campo_depois": None}

root = tk.Tk()
root.title("Alvo do Corretor (substituir)")
root.geometry("640x260+320+240")
campo = tk.Text(root, font=("Segoe UI", 12), wrap="word")
campo.pack(fill="both", expand=True, padx=8, pady=8)
campo.insert("1.0", TEXTO)

prazo = {"ate": 0.0, "fase": 0}


def preparar():
    root.lift()
    root.attributes("-topmost", True)
    root.focus_force()
    root.update()
    # volta a não ser topmost para não cobrir o balão do corretor (que é popup)
    root.attributes("-topmost", False)
    root.update()
    h = root.winfo_id()
    user32.SetForegroundWindow(h)
    campo.tag_add("sel", "1.0", "end")
    campo.focus_set()
    root.update()
    time.sleep(0.4)
    print("janela alvo (Tk) focada; texto:", repr(TEXTO))
    atalho_global()


def esperar_barra(segundos):
    limite = time.time() + segundos
    while time.time() < limite:
        if visivel(CLASSE_BARRA):
            return user32.FindWindowW(CLASSE_BARRA, None)
        root.update()
        time.sleep(0.1)
    return None


def esperar_balao_resultado(segundos):
    """Espera o balão de RESULTADO (tem 'Copiar resultado'; ignora status/prévia)."""
    limite = time.time() + segundos
    while time.time() < limite:
        b = user32.FindWindowW(CLASSE_BALAO, None)
        if b and user32.IsWindowVisible(b):
            textos = [texto_janela(f) for f in filhos(b)]
            if any(t.lower().startswith("copiar resultado") for t in textos):
                return b
        root.update()
        time.sleep(0.15)
    return None


def tick():
    if prazo["fase"] == 0:
        preparar()
        print("[1] atalho global abriu a barra?")
        prazo["ate"] = time.time() + 12
        prazo["fase"] = 1

    elif prazo["fase"] == 1:
        if time.time() > prazo["ate"]:
            resultado["erros"].append("sem_barra")
            print("  FALHA: a barra não apareceu")
            root.after(200, fim)
            return
        barra = esperar_barra(0.2)
        if barra:
            f, t = botao(barra, "Corrigir")
            if not f:
                resultado["erros"].append("sem_corrigir")
                print("  FALHA: item 'Corrigir' não encontrado")
                root.after(200, fim)
                return
            r = rect(f)
            print("  ok: barra apareceu; clicando em 'Corrigir'")
            clicar((r.left + r.right) // 2, (r.top + r.bottom) // 2)
            prazo["ate"] = time.time() + 120
            prazo["fase"] = 2

    elif prazo["fase"] == 2:
        if time.time() > prazo["ate"]:
            resultado["erros"].append("sem_balao")
            print("  FALHA: o balão de resultado não apareceu")
            root.after(200, fim)
            return
        b = esperar_balao_resultado(0.2)
        if b:
            textos = [t for t in (texto_janela(f) for f in filhos(b)) if t]
            resultado["textos_balao"] = textos
            print("  balão:", textos)
            faltando = [nome for nome in ("Substituir", "Copiar resultado", "Gerar novo texto com IA")
                        if not any(t.strip() == nome for t in textos)]
            if faltando:
                resultado["erros"].append("botoes_faltando:" + ",".join(faltando))
                print("  FALHA: faltam botões:", faltando)
                root.after(200, fim)
                return
            print("  ok: balão traz Substituir / Copiar resultado / Gerar novo texto com IA")
            try:
                from PIL import ImageGrab
                rb = rect(b)
                ImageGrab.grab(bbox=(rb.left, rb.top, rb.right, rb.bottom)).convert("RGB").save("tools/resultado.png")
                print("  foto: tools/resultado.png")
            except Exception as e:
                print("  aviso foto:", e)
            fs, _ = botao(b, "Substituir")
            rs = rect(fs)
            print("  clicando em 'Substituir'")
            clicar((rs.left + rs.right) // 2, (rs.top + rs.bottom) // 2)
            prazo["ate"] = time.time() + 8
            prazo["fase"] = 3

    elif prazo["fase"] == 3:
        if time.time() < prazo["ate"]:
            root.after(200, tick)
            return
        depois = campo.get("1.0", "end-1c")
        resultado["campo_depois"] = depois
        print("  campo depois de 'Substituir':", repr(depois))
        if depois.strip() == TEXTO or "programaçao" in depois:
            resultado["erros"].append("campo_nao_substituido")
            print("  FALHA: o campo não foi substituído")
        else:
            print("  ok: o campo foi substituído pelo texto corrigido")
        root.after(200, fim)
        return

    root.after(150, tick)


def fim():
    root.destroy()


def main():
    exe = r"target\release\corretor_universal.exe"
    if "--exe" in sys.argv:
        exe = sys.argv[sys.argv.index("--exe") + 1]

    if not app_rodando():
        print("iniciando o app:", exe)
        log = open("tools/dbg.log", "w", encoding="utf-8")
        subprocess.Popen([exe], stderr=log)
        time.sleep(4.0)

    root.after(3000, tick)
    root.mainloop()

    print()
    if not resultado["erros"]:
        print("RESULTADO: OK - o botão 'Substituir' do balão cola o texto no campo")
        return 0
    print("RESULTADO: FALHA -", resultado["erros"])
    return 1


if __name__ == "__main__":
    sys.exit(main())
