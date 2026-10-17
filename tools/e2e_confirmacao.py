"""Teste real do balão de prévia/confirmação (fluxo de ponta a ponta).

Cria uma janela Win32 própria com um EDIT contendo texto com erros, seleciona
tudo, aciona a barra pelo atalho global (Ctrl+Alt+C) e clica no rótulo
"Corrigir" — que deve executar a correção e mostrar o balão de prévia.

Confere:
1. o rótulo "Corrigir" dispara a correção direto (não abre menu);
2. o balão (CorretorBalaoPopup) traz "Confirmar substituição" e "Cancelar";
3. o campo ainda tem o texto antigo antes de confirmar;
4. depois de "Confirmar substituição" o campo passa a ter o texto corrigido;
5. a seta (▾) do "Corrigir" abre o flyout com as variantes.

Uso: python tools/e2e_confirmacao.py [--exe CAMINHO]
"""
import ctypes
import subprocess
import sys
import threading
import time
from ctypes import wintypes

sys.stdout.reconfigure(encoding="utf-8", errors="replace")

user32 = ctypes.windll.user32
kernel32 = ctypes.windll.kernel32
gdi32 = ctypes.windll.gdi32

user32.FindWindowW.restype = wintypes.HWND
user32.FindWindowW.argtypes = [wintypes.LPCWSTR, wintypes.LPCWSTR]
user32.IsWindowVisible.argtypes = [wintypes.HWND]
user32.CreateWindowExW.restype = wintypes.HWND
user32.CreateWindowExW.argtypes = [wintypes.DWORD, wintypes.LPCWSTR, wintypes.LPCWSTR, wintypes.DWORD,
                                   ctypes.c_int, ctypes.c_int, ctypes.c_int, ctypes.c_int,
                                   wintypes.HWND, wintypes.HMENU, wintypes.HINSTANCE, wintypes.LPVOID]
user32.SendMessageW.restype = ctypes.c_ssize_t
user32.SendMessageW.argtypes = [wintypes.HWND, ctypes.c_uint, wintypes.WPARAM, wintypes.LPARAM]
user32.DefWindowProcW.restype = ctypes.c_ssize_t
user32.DefWindowProcW.argtypes = [wintypes.HWND, ctypes.c_uint, wintypes.WPARAM, wintypes.LPARAM]
user32.GetWindowTextW.restype = ctypes.c_int
kernel32.GetModuleHandleW.restype = wintypes.HMODULE
gdi32.CreateFontW.restype = wintypes.HANDLE

CLASSE_BARRA = "CorretorMenuPopup"
CLASSE_FLYOUT = "CorretorFlyoutPopup"
CLASSE_BALAO = "CorretorBalaoPopup"

TEXTO = "Eu gosto de programaçao em Pyton ."
ESPERADO = "Eu gosto de programação em Pyton."

WM_SETFONT = 0x0030
EM_SETSEL = 0x00B1
ES_MULTILINE = 0x0004
ES_AUTOVSCROLL = 0x0040
WS_CHILD = 0x40000000
WS_VISIBLE = 0x10000000
WS_BORDER = 0x00800000
WS_VSCROLL = 0x00200000
WS_TABSTOP = 0x00010000
WS_OVERLAPPEDWINDOW = 0x00CF0000

WNDPROC = ctypes.WINFUNCTYPE(ctypes.c_ssize_t, wintypes.HWND, ctypes.c_uint, wintypes.WPARAM, wintypes.LPARAM)
_referencias = []


class KEYBDINPUT(ctypes.Structure):
    _fields_ = [("wVk", wintypes.WORD), ("wScan", wintypes.WORD), ("dwFlags", wintypes.DWORD),
                ("time", wintypes.DWORD), ("dwExtraInfo", ctypes.c_void_p)]


class _ENTRADA(ctypes.Union):
    _fields_ = [("ki", KEYBDINPUT), ("padding", ctypes.c_byte * 32)]


class INPUT(ctypes.Structure):
    _fields_ = [("type", wintypes.DWORD), ("u", _ENTRADA)]


class WNDCLASSW(ctypes.Structure):
    _fields_ = [("style", ctypes.c_uint), ("lpfnWndProc", WNDPROC),
                ("cbClsExtra", ctypes.c_int), ("cbWndExtra", ctypes.c_int),
                ("hInstance", wintypes.HINSTANCE), ("hIcon", wintypes.HICON),
                ("hCursor", wintypes.HANDLE), ("hbrBackground", wintypes.HBRUSH),
                ("lpszMenuName", wintypes.LPCWSTR), ("lpszClassName", wintypes.LPCWSTR)]


user32.SendInput.restype = wintypes.UINT
user32.SendInput.argtypes = [wintypes.UINT, ctypes.POINTER(INPUT), ctypes.c_int]


def tecla(vk, soltar=False):
    entrada = INPUT()
    entrada.type = 1
    entrada.u.ki = KEYBDINPUT(wVk=vk, wScan=0, dwFlags=2 if soltar else 0, time=0, dwExtraInfo=None)
    user32.SendInput(1, ctypes.byref(entrada), ctypes.sizeof(INPUT))
    time.sleep(0.03)


def atalho_global():
    tecla(0x11)
    tecla(0x12)
    tecla(0x43)
    tecla(0x43, True)
    tecla(0x12, True)
    tecla(0x11, True)


def esc():
    tecla(0x1B)
    tecla(0x1B, True)
    time.sleep(0.4)


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


def esperar(classe, segundos=5.0):
    limite = time.time() + segundos
    while time.time() < limite:
        hwnd = user32.FindWindowW(classe, None)
        if hwnd and user32.IsWindowVisible(hwnd):
            return hwnd
        time.sleep(0.1)
    return None


def esperar_confirmacao(segundos=90.0):
    """Espera o balão de PRÉVIA (ignora o de status 'aguardando IA')."""
    limite = time.time() + segundos
    while time.time() < limite:
        b = user32.FindWindowW(CLASSE_BALAO, None)
        if b and user32.IsWindowVisible(b):
            textos = [texto_janela(f) for f in filhos(b)]
            if any(t.startswith("Confirmar substituição") for t in textos):
                return b
        time.sleep(0.15)
    return None


def criar_janela_alvo():
    hinst = kernel32.GetModuleHandleW(None)
    proc = WNDPROC(lambda h, m, w, l: user32.DefWindowProcW(h, m, w, l))
    _referencias.append(proc)
    wc = WNDCLASSW()
    wc.lpfnWndProc = proc
    wc.hInstance = hinst
    wc.lpszClassName = "CorretorTesteHost"
    user32.RegisterClassW(ctypes.byref(wc))

    hwnd = user32.CreateWindowExW(
        0, "CorretorTesteHost", "Alvo do Corretor (teste)",
        WS_OVERLAPPEDWINDOW, 300, 220, 620, 240, None, None, hinst, None)
    edit = user32.CreateWindowExW(
        0, "EDIT", TEXTO,
        WS_CHILD | WS_VISIBLE | WS_BORDER | WS_VSCROLL | WS_TABSTOP
        | ES_MULTILINE | ES_AUTOVSCROLL,
        10, 10, 590, 180, hwnd, None, hinst, None)
    fonte = gdi32.CreateFontW(-18, 0, 0, 0, 400, 0, 0, 0, 0, 0, 0, 0, 0, "Segoe UI")
    user32.SendMessageW(edit, WM_SETFONT, fonte, 1)
    user32.ShowWindow(hwnd, 5)
    user32.ShowWindow(edit, 5)
    user32.UpdateWindow(hwnd)
    user32.SetForegroundWindow(hwnd)
    user32.SetFocus(edit)
    user32.SendMessageW(edit, EM_SETSEL, 0, -1)
    return hwnd, edit


def seleciona_tudo(edit):
    user32.SetFocus(edit)
    user32.SendMessageW(edit, EM_SETSEL, 0, -1)


resultado = {"ok": False}


def abrir_barra():
    seleciona_tudo(EDICAO[0])
    time.sleep(0.6)
    atalho_global()
    time.sleep(0.8)
    return esperar(CLASSE_BARRA, 10.0)


def teste_rotulo_dispara_correcao():
    """Clica no rótulo 'Corrigir' e espera o balão de confirmação."""
    barra = abrir_barra()
    if not barra:
        return "sem_barra"
    time.sleep(1.0)
    f, t = botao(barra, "Corrigir")
    if not f:
        return "sem_corrigir"
    r = rect(f)
    print("  clicando no rótulo %r em %s" % (t, (r.left, r.top, r.right, r.bottom)))
    clicar((r.left + r.right) // 2, (r.top + r.bottom) // 2)

    # LanguageTool + IA podem levar alguns segundos; primeiro aparece o balão
    # "aguardando IA" e depois o de prévia.
    balao = esperar_confirmacao(90.0)
    if not balao:
        return "sem_balao"
    time.sleep(0.3)
    textos = [texto_janela(f) for f in filhos(balao)]
    textos = [t for t in textos if t]
    resultado["textos_balao"] = textos
    print("  balão:", textos)
    if not any("Confirmar substituição" in t for t in textos):
        return "sem_confirmar"
    if not any(t.strip() == "Cancelar" for t in textos):
        return "sem_cancelar"

    antes = texto_janela(EDICAO[0])
    print("  campo antes de confirmar :", repr(antes))
    if antes.strip() != TEXTO:
        return "campo_mudou_antes"

    try:
        from PIL import ImageGrab
        rb = rect(balao)
        ImageGrab.grab(bbox=(rb.left, rb.top, rb.right, rb.bottom)).convert("RGB").save("tools/confirmacao.png")
        print("  foto: tools/confirmacao.png")
    except Exception as e:
        print("  aviso foto:", e)

    fc, _ = botao(balao, "Confirmar substituição")
    if not fc:
        return "sem_botao_confirmar"
    rc = rect(fc)
    rb = rect(balao)
    largura_tela = user32.GetSystemMetrics(0)
    altura_tela = user32.GetSystemMetrics(1)
    print("  balão rect:", (rb.left, rb.top, rb.right, rb.bottom))
    print("  tela: %dx%d" % (largura_tela, altura_tela))
    print("  botão confirmar rect:", (rc.left, rc.top, rc.right, rc.bottom))
    cx, cy = (rc.left + rc.right) // 2, (rc.top + rc.bottom) // 2
    user32.WindowFromPoint.restype = wintypes.HWND
    sob = user32.WindowFromPoint(wintypes.POINT(cx, cy))
    print("  janela sob o clique: hwnd=%s (confirmar=%s balão=%s)" % (sob, fc, balao))
    clicar(cx, cy)
    time.sleep(1.5)
    time.sleep(1.2)

    depois = texto_janela(EDICAO[0])
    print("  campo depois de confirmar:", repr(depois))
    if depois.strip() == TEXTO or "programaçao" in depois:
        return "campo_nao_substituido"
    return None


def teste_seta_abre_flyout():
    barra = abrir_barra()
    if not barra:
        return "sem_barra"
    f, _ = botao(barra, "Corrigir")
    if not f:
        return "sem_corrigir"
    r = rect(f)
    print("  clicando na seta em (%d, %d)" % (r.right - 8, (r.top + r.bottom) // 2))
    clicar(r.right - 8, (r.top + r.bottom) // 2)
    flyout = esperar(CLASSE_FLYOUT, 10.0)
    if not flyout:
        return "sem_flyout"
    itens = [texto_janela(f) for f in filhos(flyout)]
    itens = [t for t in itens if t]
    print("  flyout:", itens)
    if not any("Substituir a seleção" in t for t in itens):
        return "flyout_sem_substituir"
    esc()
    return None


EDICAO = [None]

erros = []


def automatizar(edit):
    global erros
    EDICAO[0] = edit
    time.sleep(3.0)

    print("[1] rótulo 'Corrigir' dispara a correção e mostra o balão")
    e = teste_rotulo_dispara_correcao()
    if e:
        erros.append(("rotulo->balão", e))
        print("  FALHA:", e)
    else:
        print("  ok")

    esc()
    time.sleep(1.5)

    if "--passo1" in sys.argv:
        return

    print("[2] seta (▾) do 'Corrigir' abre o flyout de variantes")
    e = teste_seta_abre_flyout()
    if e:
        erros.append(("seta->flyout", e))
        print("  FALHA:", e)
    else:
        print("  ok")


def bombeia(segundos):
    msg = wintypes.MSG()
    limite = time.time() + segundos
    while time.time() < limite:
        while user32.PeekMessageW(ctypes.byref(msg), None, 0, 0, 1):
            user32.TranslateMessage(ctypes.byref(msg))
            user32.DispatchMessageW(ctypes.byref(msg))
        time.sleep(0.01)


def main():
    hwnd, edit = criar_janela_alvo()
    print("janela alvo criada; texto:", repr(TEXTO))

    t = threading.Thread(target=automatizar, args=(edit,), daemon=True)
    t.start()

    limite = time.time() + 90
    while t.is_alive() and time.time() < limite:
        bombeia(0.3)

    print()
    if not erros:
        print("RESULTADO: OK - o rótulo Corrigir mostra o balão de confirmação e a substituição é aplicada")
        return 0
    print("RESULTADO: FALHA -", erros)
    return 1


if __name__ == "__main__":
    sys.exit(main())
