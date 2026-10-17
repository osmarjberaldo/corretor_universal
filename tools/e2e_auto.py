"""Testa a detecção automática (arrasto do mouse) e o destravamento após
dispensar uma prévia.

Passos:
1. cria uma janela com EDIT e seleciona texto com um arrasto real do mouse;
   a barra de ações deve aparecer sozinha (deteccao automatica);
2. dispara uma prévia pelo atalho e o "Corrigir" do app;
3. dispensa o balão de prévia com ESC;
4. faz o arrasto de novo: a barra DEVE voltar a aparecer (prova de que a
   prévia descartada não trava mais a detecção).

Uso: python tools/e2e_auto.py [--exe CAMINHO]
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
CLASSE_BALAO = "CorretorBalaoPopup"

TEXTO = "Eu gosto de programaçao em Pyton ."

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
    time.sleep(0.5)


def arrastar(x1, y1, x2, y2):
    user32.SetCursorPos(x1, y1)
    time.sleep(0.2)
    user32.mouse_event(0x0002, 0, 0, 0, 0)  # LEFTDOWN
    time.sleep(0.15)
    passos = 12
    for i in range(1, passos + 1):
        user32.SetCursorPos(x1 + (x2 - x1) * i // passos, y1 + (y2 - y1) * i // passos)
        time.sleep(0.03)
    time.sleep(0.1)
    user32.mouse_event(0x0004, 0, 0, 0, 0)  # LEFTUP


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


def esperar(classe, segundos=5.0):
    limite = time.time() + segundos
    while time.time() < limite:
        if visivel(classe):
            return user32.FindWindowW(classe, None)
        time.sleep(0.1)
    return None


def esperar_confirmacao(segundos=90.0):
    """Espera o balão de PRÉVIA (não o de status 'aguardando IA')."""
    limite = time.time() + segundos
    while time.time() < limite:
        b = user32.FindWindowW(CLASSE_BALAO, None)
        if b and user32.IsWindowVisible(b):
            textos = [texto_janela(f) for f in filhos(b)]
            if any(t.startswith("Confirmar substituição") for t in textos):
                return b
        time.sleep(0.15)
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
    wc.lpszClassName = "CorretorTesteHostAuto"
    user32.RegisterClassW(ctypes.byref(wc))

    hwnd = user32.CreateWindowExW(
        0, "CorretorTesteHostAuto", "Alvo do Corretor (auto)",
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
    return hwnd, edit


EDICAO = [None]
EDICAO_HOST = [None]
erros = []


def regiao_texto(edit):
    r = rect(edit)
    x1 = r.left + 30
    y1 = r.top + 20
    return x1, y1, x1 + 160, y1


def focar():
    h = EDICAO[0]
    user32.ShowWindow(EDICAO_HOST[0], 5)
    user32.SetForegroundWindow(h)
    user32.BringWindowToTop(h)
    user32.SetFocus(h)
    time.sleep(0.3)


def arrasto_ate_barra():
    """Faz o arrasto (com novas tentativas) até a barra aparecer."""
    for tentativa in range(3):
        focar()
        esc()
        time.sleep(0.3)
        user32.SetFocus(EDICAO[0])
        user32.SendMessageW(EDICAO[0], EM_SETSEL, 0, 0)
        time.sleep(0.3)
        x1, y1, x2, y2 = regiao_texto(EDICAO[0])
        arrastar(x1, y1, x2, y2)
        if esperar(CLASSE_BARRA, 4.0):
            return True
        print("    (tentativa %d não abriu a barra)" % (tentativa + 1))
    return False


def teste_auto():
    print("[1] arrasto do mouse deve abrir a barra sozinho")
    if not arrasto_ate_barra():
        return "barra_nao_apareceu_no_arrasto"
    print("  ok: barra apareceu no arrasto")
    esc()
    time.sleep(0.6)

    print("[2] dispara uma prévia pelo atalho + 'Corrigir'")
    focar()
    user32.SendMessageW(EDICAO[0], EM_SETSEL, 0, -1)
    time.sleep(0.5)
    atalho_global()
    barra = esperar(CLASSE_BARRA, 8.0)
    if not barra:
        return "sem_barra_atalho"
    time.sleep(0.8)
    f, _ = botao(barra, "Corrigir")
    if not f:
        return "sem_corrigir"
    r = rect(f)
    user32.SetCursorPos((r.left + r.right) // 2, (r.top + r.bottom) // 2)
    time.sleep(0.3)
    user32.mouse_event(0x0002, 0, 0, 0, 0)
    time.sleep(0.15)
    user32.mouse_event(0x0004, 0, 0, 0, 0)
    if not esperar_confirmacao(90.0):
        return "sem_balao_previa"
    print("  ok: balão de prévia apareceu")

    print("[3] ESC dispensa a prévia sem confirmar")
    esc()
    time.sleep(0.8)
    if visivel(CLASSE_BALAO):
        return "balao_nao_fechou_com_esc"
    print("  ok: balão fechou")

    print("[4] novo arrasto deve reabrir a barra (prévia descartada destravou)")
    if not arrasto_ate_barra():
        return "detectcao_travada_apos_previa"
    print("  ok: barra voltou a aparecer")
    esc()
    return None


def automatizar(edit):
    EDICAO[0] = edit
    time.sleep(3.0)
    e = teste_auto()
    if e:
        erros.append(e)
        print("  FALHA:", e)


def bombeia(segundos):
    msg = wintypes.MSG()
    limite = time.time() + segundos
    while time.time() < limite:
        while user32.PeekMessageW(ctypes.byref(msg), None, 0, 0, 1):
            user32.TranslateMessage(ctypes.byref(msg))
            user32.DispatchMessageW(ctypes.byref(msg))
        time.sleep(0.01)


def main():
    exe = r"target\release\corretor_universal.exe"
    if "--exe" in sys.argv:
        exe = sys.argv[sys.argv.index("--exe") + 1]

    if not app_rodando():
        print("iniciando o app:", exe)
        log = open("tools/dbg.log", "w", encoding="utf-8")
        subprocess.Popen([exe], stderr=log)
        time.sleep(4.0)

    hwnd, edit = criar_janela_alvo()
    EDICAO_HOST[0] = hwnd
    print("janela alvo criada; texto:", repr(TEXTO))

    t = threading.Thread(target=automatizar, args=(edit,), daemon=True)
    t.start()

    limite = time.time() + 140
    while t.is_alive() and time.time() < limite:
        bombeia(0.3)

    try:
        with open("tools/dbg.log", "r", encoding="utf-8", errors="replace") as arq:
            linhas = [l.rstrip() for l in arq if l.strip()]
        if linhas:
            print()
            print("--- log de depuração do app ---")
            for l in linhas[-40:]:
                print(l)
    except FileNotFoundError:
        pass

    print()
    if not erros:
        print("RESULTADO: OK - detecção automática funciona e destrava após dispensar a prévia")
        return 0
    print("RESULTADO: FALHA -", erros)
    return 1


if __name__ == "__main__":
    sys.exit(main())
