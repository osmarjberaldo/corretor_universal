"""Teste do item "Corrigir com IA (gramática) e substituir" (ponta a ponta).

Cria uma janela Win32 com um EDIT contendo um texto com erro de gramática,
seleciona tudo, abre a barra (atalho Ctrl+Alt+C), clica na seta do "Corrigir",
escolhe "Corrigir com IA (gramática) e substituir" e inspeciona o balão de
prévia (o texto novo que a IA devolveu) e o campo após confirmar.

Uso: python tools/e2e_ia.py [--exe CAMINHO] [--texto "..."]

Sobe o app sozinho se ele ainda não estiver rodando.
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
kernel32.GetModuleHandleW.restype = wintypes.HMODULE
gdi32.CreateFontW.restype = wintypes.HANDLE

CLASSE_BARRA = "CorretorMenuPopup"
CLASSE_FLYOUT = "CorretorFlyoutPopup"
CLASSE_BALAO = "CorretorBalaoPopup"

TEXTO = "Bom dia, como vai vc"

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


def classe_janela(hwnd):
    buf = ctypes.create_unicode_buffer(256)
    user32.GetClassNameW(hwnd, buf, 256)
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


def conteudo_edits(janela):
    return [texto_janela(f) for f in filhos(janela) if classe_janela(f).upper().startswith("EDIT")]


def esperar(classe, segundos=5.0):
    limite = time.time() + segundos
    while time.time() < limite:
        hwnd = user32.FindWindowW(classe, None)
        if hwnd and user32.IsWindowVisible(hwnd):
            return hwnd
        time.sleep(0.1)
    return None


def app_rodando():
    out = subprocess.run(["tasklist", "/FI", "IMAGENAME eq corretor_universal.exe", "/FO", "CSV", "/NH"],
                         capture_output=True, text=True).stdout
    return "corretor_universal.exe" in out


def criar_janela_alvo():
    hinst = kernel32.GetModuleHandleW(None)
    proc = WNDPROC(lambda h, m, w, l: user32.DefWindowProcW(h, m, w, l))
    _referencias.append(proc)
    wc = WNDCLASSW()
    wc.lpfnWndProc = proc
    wc.hInstance = hinst
    wc.lpszClassName = "CorretorTesteHostIa"
    user32.RegisterClassW(ctypes.byref(wc))

    hwnd = user32.CreateWindowExW(
        0, "CorretorTesteHostIa", "Alvo do Corretor (IA)",
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


EDICAO = [None]
erros = []


def abrir_barra():
    seleciona_tudo(EDICAO[0])
    time.sleep(0.6)
    atalho_global()
    time.sleep(1.2)
    return esperar(CLASSE_BARRA, 10.0)


def teste_corrigir_com_ia():
    barra = abrir_barra()
    if not barra:
        return "sem_barra"
    f, _ = botao(barra, "Corrigir")
    if not f:
        return "sem_corrigir"
    r = rect(f)
    print("  clicando na seta do Corrigir em (%d, %d)" % (r.right - 8, (r.top + r.bottom) // 2))
    clicar(r.right - 8, (r.top + r.bottom) // 2)
    flyout = esperar(CLASSE_FLYOUT, 10.0)
    if not flyout:
        return "sem_flyout"
    itens = [t for t in (texto_janela(x) for x in filhos(flyout)) if t]
    print("  flyout:", itens)

    alvo, rotulo = botao(flyout, "Corrigir com IA")
    if not alvo:
        return "sem_item_ia"
    ri = rect(alvo)
    print("  clicando em %r" % rotulo)
    clicar((ri.left + ri.right) // 2, (ri.top + ri.bottom) // 2)

    # Enquanto a IA responde deve aparecer o balão "aguardando IA"; depois ele é
    # trocado pelo balão de prévia com o texto corrigido.
    balao = None
    vistos = []
    limite = time.time() + 90
    while time.time() < limite:
        b = user32.FindWindowW(CLASSE_BALAO, None)
        if b and user32.IsWindowVisible(b):
            balao = b
            snap = [texto_janela(x) for x in filhos(b)]
            snap = [t for t in snap if t]
            if not vistos or vistos[-1] != snap:
                vistos.append(snap)
                print("  balão agora:", snap)
            if any("Confirmar substituição" in t for t in snap):
                break
        time.sleep(0.15)

    resultado["sequencia_balao"] = vistos
    if balao is None:
        return "sem_balao"
    if not any(any("Aguardando a IA" in t for t in snap) for snap in vistos):
        print("  aviso: não vi o balão 'aguardando IA' (resposta muito rápida?)")
    time.sleep(0.4)
    textos = [texto_janela(x) for x in filhos(balao)]
    textos = [t for t in textos if t]
    edits = conteudo_edits(balao)
    resultado["textos_balao"] = textos
    resultado["edits_balao"] = edits
    print("  textos do balão:", textos)
    print("  edit do balão  :", edits)

    if not any("Confirmar substituição" in t for t in textos):
        return "sem_confirmar"

    fc, _ = botao(balao, "Confirmar substituição")
    rc = rect(fc)
    clicar((rc.left + rc.right) // 2, (rc.top + rc.bottom) // 2)
    time.sleep(2.5)

    depois = texto_janela(EDICAO[0])
    resultado["campo_depois"] = depois
    print("  campo depois:", repr(depois))
    if depois.strip() == TEXTO:
        return "campo_nao_substituido"
    if "vc" in depois:
        return "gramatica_nao_corrigida"
    return None


resultado = {}


def automatizar(edit):
    EDICAO[0] = edit
    time.sleep(3.0)
    print("[IA] Corrigir com IA (gramática) e substituir")
    e = teste_corrigir_com_ia()
    if e:
        erros.append(("corrigir_com_ia", e))
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
    global TEXTO
    exe = r"target\release\corretor_universal.exe"
    if "--exe" in sys.argv:
        exe = sys.argv[sys.argv.index("--exe") + 1]
    if "--texto" in sys.argv:
        TEXTO = sys.argv[sys.argv.index("--texto") + 1]

    if not app_rodando():
        print("iniciando o app:", exe)
        subprocess.Popen([exe])
        time.sleep(4.0)
    else:
        print("app já estava rodando")

    hwnd, edit = criar_janela_alvo()
    print("janela alvo criada; texto:", repr(TEXTO))

    t = threading.Thread(target=automatizar, args=(edit,), daemon=True)
    t.start()

    limite = time.time() + 140
    while t.is_alive() and time.time() < limite:
        bombeia(0.3)

    print()
    print("resultado:", resultado)
    if not erros:
        print("RESULTADO: OK - Corrigir com IA corrigiu a gramática e substituiu")
        return 0
    print("RESULTADO: FALHA -", erros)
    return 1


if __name__ == "__main__":
    sys.exit(main())
