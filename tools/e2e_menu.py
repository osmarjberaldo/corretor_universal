"""Testa o menu que aparece sozinho ao selecionar texto.

O script:
1. abre uma janela de teste com um EDIT contendo texto com erros;
2. faz um arrasto real com o mouse (SetCursorPos + mouse_event) para selecionar;
3. espera a janela de classe "CorretorMenuPopup" aparecer e lê os itens;
4. clica em "Corrigir" com o mouse (clique real, sem ativar a janela do menu);
5. confere se o texto selecionado foi substituído e se o foco nunca saiu da
   janela de teste (requisito do design: o popup não pode roubar o foco).

Uso:
    python tools/e2e_menu.py [texto] [--somente-leitura] [--atalho] [--melhorar]
    python tools/e2e_menu.py --pela-seta        (Corrigir > Apenas copiar a correção)
    python tools/e2e_menu.py --melhorar-formal  (Melhorar > Mais formal)
    python tools/e2e_menu.py --copiar           (botão ⧉)
    python tools/e2e_menu.py --sugestao=N       (N-ésima alternativa real na seta do Corrigir)
    python tools/e2e_menu.py --revisar          ("Revisar avisos…" abre a janela)
    python tools/e2e_menu.py "palavra" --dicionario   (⋮ > Adicionar seleção ao dicionário)
"""
import ctypes
import json
import os
import sys
import tempfile
import threading
import time
from ctypes import wintypes


def _caminho_config():
    base = os.environ.get("APPDATA") or os.path.expanduser("~/.config")
    return os.path.join(base, "corretor_universal", "config.json")


CAMINHO_CONFIG = _caminho_config()

sys.stdout.reconfigure(encoding="utf-8", errors="replace")

user32 = ctypes.windll.user32
kernel32 = ctypes.windll.kernel32

TEXTO = sys.argv[1] if len(sys.argv) > 1 and not sys.argv[1].startswith("--") else \
    "Eu gosto de programaçao em Rust e vc sabe isso."
SOMENTE_LEITURA = "--somente-leitura" in sys.argv
# --atalho: em vez do arrasto, seleciona tudo e dispara o atalho global
ATALHO = "--atalho" in sys.argv
# --melhorar: usa a ação de IA em vez de Corrigir
MELHORAR = "--melhorar" in sys.argv
# "Melhorar > Mais formal": clique na seta e escolha no flyout (a IA recebe
# outra instrução; o mock registra a requisição para conferirmos)
MELHORAR_FORMAL = "--melhorar-formal" in sys.argv
# "Corrigir > Apenas copiar a correção" (clique na seta, escolha no flyout)
PELA_SETA = "--pela-seta" in sys.argv or MELHORAR_FORMAL
# "⋮ > Adicionar seleção ao dicionário" (com uma palavra selecionada)
DICIONARIO = "--dicionario" in sys.argv
# --sugestao=N : escolhe a N-ésima alternativa real na seta do "Corrigir"
SUGESTAO = None
for _arg in sys.argv:
    if _arg.startswith("--sugestao="):
        SUGESTAO = int(_arg.split("=", 1)[1])
# --revisar : clica em "Revisar avisos…" e confere que a janela abre
REVISAR = "--revisar" in sys.argv
# "Copiar" (só o ícone): copia a seleção sem chamar a API
COPIAR = "--copiar" in sys.argv
ROTULO_ACAO = "copiar" if COPIAR else ("melhorar" if (MELHORAR or MELHORAR_FORMAL) else "corrigir")
if MELHORAR_FORMAL:
    VARIANTE = "Mais formal"
elif PELA_SETA:
    VARIANTE = "Apenas copiar"
else:
    VARIANTE = ""

LOG_MOCK = os.path.join(tempfile.gettempdir(), "mock_ia_requests.log")
EM_SETSEL = 0x00B1
VK_CONTROL, VK_C = 0x11, 0x43
KEYEVENTF_KEYUP = 0x0002

WM_SETTEXT, WM_GETTEXT, WM_GETTEXTLENGTH = 0x000C, 0x000D, 0x000E
WM_CLOSE, WM_DESTROY = 0x0010, 0x0002
WS_CHILD, WS_VISIBLE, WS_VSCROLL, WS_TABSTOP, WS_POPUP = 0x40000000, 0x10000000, 0x00200000, 0x00010000, 0x80000000
WS_EX_CLIENTEDGE, WS_EX_TOOLWINDOW = 0x00000200, 0x00000080
ES_MULTILINE, ES_AUTOVSCROLL, ES_READONLY = 0x0004, 0x0040, 0x0800
MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP = 0x0002, 0x0004
VK_MENU = 0x12
SW_SHOW = 5
SW_RESTORE = 9

user32.CreateWindowExW.restype = wintypes.HWND
user32.CreateWindowExW.argtypes = [wintypes.DWORD, wintypes.LPCWSTR, wintypes.LPCWSTR, wintypes.DWORD,
                                   ctypes.c_int, ctypes.c_int, ctypes.c_int, ctypes.c_int,
                                   wintypes.HWND, wintypes.HMENU, wintypes.HINSTANCE, ctypes.c_void_p]
user32.DefWindowProcW.restype = wintypes.LPARAM
user32.DefWindowProcW.argtypes = [wintypes.HWND, wintypes.UINT, wintypes.WPARAM, wintypes.LPARAM]
user32.SendMessageW.restype = wintypes.LPARAM
user32.SendMessageW.argtypes = [wintypes.HWND, wintypes.UINT, wintypes.WPARAM, wintypes.LPARAM]
user32.GetMessageW.argtypes = [ctypes.c_void_p, wintypes.HWND, wintypes.UINT, wintypes.UINT]
user32.FindWindowW.restype = wintypes.HWND
user32.FindWindowW.argtypes = [wintypes.LPCWSTR, wintypes.LPCWSTR]
user32.IsWindowVisible.argtypes = [wintypes.HWND]
user32.GetWindowRect.argtypes = [wintypes.HWND, ctypes.POINTER(wintypes.RECT)]
user32.GetWindowTextW.argtypes = [wintypes.HWND, wintypes.LPWSTR, ctypes.c_int]
user32.GetWindowTextLengthW.argtypes = [wintypes.HWND]
user32.GetForegroundWindow.restype = wintypes.HWND
user32.GetWindowThreadProcessId.argtypes = [wintypes.HWND, ctypes.POINTER(wintypes.DWORD)]
user32.AttachThreadInput.argtypes = [wintypes.DWORD, wintypes.DWORD, wintypes.BOOL]
user32.SetForegroundWindow.argtypes = [wintypes.HWND]
user32.SetFocus.argtypes = [wintypes.HWND]
user32.SetCursorPos.argtypes = [ctypes.c_int, ctypes.c_int]
user32.mouse_event.argtypes = [wintypes.DWORD, wintypes.DWORD, wintypes.DWORD, wintypes.DWORD, ctypes.c_void_p]
kernel32.GetModuleHandleW.restype = wintypes.HMODULE
kernel32.GetCurrentThreadId.restype = wintypes.DWORD


CF_UNICODETEXT, GMEM_MOVEABLE = 13, 0x0002
user32.OpenClipboard.argtypes = [wintypes.HWND]
user32.OpenClipboard.restype = wintypes.BOOL
user32.EmptyClipboard.restype = wintypes.BOOL
user32.CloseClipboard.restype = wintypes.BOOL
user32.GetClipboardData.restype = wintypes.HANDLE
user32.GetClipboardData.argtypes = [wintypes.UINT]
user32.SetClipboardData.restype = wintypes.HANDLE
user32.SetClipboardData.argtypes = [wintypes.UINT, wintypes.HANDLE]
kernel32.GlobalAlloc.restype = wintypes.HGLOBAL
kernel32.GlobalAlloc.argtypes = [wintypes.UINT, ctypes.c_size_t]
kernel32.GlobalLock.restype = ctypes.c_void_p
kernel32.GlobalLock.argtypes = [wintypes.HGLOBAL]
kernel32.GlobalUnlock.argtypes = [wintypes.HGLOBAL]


def clip_get():
    if not user32.OpenClipboard(None):
        return "<ocupado>"
    try:
        handle = user32.GetClipboardData(CF_UNICODETEXT)
        if not handle:
            return None
        ptr = kernel32.GlobalLock(handle)
        if not ptr:
            return None
        try:
            return ctypes.c_wchar_p(ptr).value
        finally:
            kernel32.GlobalUnlock(handle)
    finally:
        user32.CloseClipboard()


def clip_set(texto):
    payload = texto.encode("utf-16-le") + b"\x00\x00"
    handle = kernel32.GlobalAlloc(GMEM_MOVEABLE, len(payload))
    ptr = kernel32.GlobalLock(handle)
    ctypes.memmove(ptr, payload, len(payload))
    kernel32.GlobalUnlock(handle)
    for _ in range(40):
        if user32.OpenClipboard(None):
            user32.EmptyClipboard()
            ok = user32.SetClipboardData(CF_UNICODETEXT, handle)
            user32.CloseClipboard()
            return bool(ok)
        time.sleep(0.05)
    return False


class WNDCLASSEXW(ctypes.Structure):
    _fields_ = [("cbSize", wintypes.UINT), ("style", wintypes.UINT), ("lpfnWndProc", ctypes.c_void_p),
                ("cbClsExtra", ctypes.c_int), ("cbWndExtra", ctypes.c_int), ("hInstance", wintypes.HINSTANCE),
                ("hIcon", wintypes.HICON), ("hCursor", ctypes.c_void_p), ("hbrBackground", ctypes.c_void_p),
                ("lpszMenuName", wintypes.LPCWSTR), ("lpszClassName", wintypes.LPCWSTR), ("hIconSm", wintypes.HICON)]


class MSG(ctypes.Structure):
    _fields_ = [("hwnd", wintypes.HWND), ("message", wintypes.UINT), ("wParam", wintypes.WPARAM),
                ("lParam", wintypes.LPARAM), ("time", wintypes.DWORD), ("pt", wintypes.POINT)]


EDIT = None
janela = None
resultados = {}


def texto_janela(hwnd):
    n = user32.GetWindowTextLengthW(hwnd)
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


def edit_read(edit):
    n = user32.SendMessageW(edit, WM_GETTEXTLENGTH, 0, 0)
    buf = ctypes.create_unicode_buffer(n + 1)
    user32.SendMessageW(edit, WM_GETTEXT, n + 1, ctypes.addressof(buf))
    return buf.value


def focar(hwnd, controle):
    user32.keybd_event(VK_MENU, 0, 0, 0)
    user32.keybd_event(VK_MENU, 0, 2, 0)
    alvo = user32.GetWindowThreadProcessId(hwnd, None)
    atual = kernel32.GetCurrentThreadId()
    user32.AttachThreadInput(atual, alvo, True)
    try:
        user32.ShowWindow(hwnd, SW_RESTORE)
        user32.SetForegroundWindow(hwnd)
        user32.SetFocus(controle)
    finally:
        user32.AttachThreadInput(atual, alvo, False)
    time.sleep(0.5)
    return user32.GetForegroundWindow() == hwnd


class KEYBDINPUT(ctypes.Structure):
    _fields_ = [("wVk", wintypes.WORD), ("wScan", wintypes.WORD), ("dwFlags", wintypes.DWORD),
                ("time", wintypes.DWORD), ("dwExtraInfo", ctypes.c_void_p)]


class _INPUTUNION(ctypes.Union):
    _fields_ = [("ki", KEYBDINPUT), ("padding", ctypes.c_byte * 32)]


class INPUT(ctypes.Structure):
    _fields_ = [("type", wintypes.DWORD), ("u", _INPUTUNION)]


user32.SendInput.restype = wintypes.UINT
user32.SendInput.argtypes = [wintypes.UINT, ctypes.POINTER(INPUT), ctypes.c_int]


def tecla(vk, soltar=False):
    entrada = INPUT()
    entrada.type = 1  # INPUT_KEYBOARD
    entrada.u.ki = KEYBDINPUT(wVk=vk, wScan=0, dwFlags=KEYEVENTF_KEYUP if soltar else 0,
                              time=0, dwExtraInfo=None)
    user32.SendInput(1, ctypes.byref(entrada), ctypes.sizeof(INPUT))
    time.sleep(0.03)


def atalho_global():
    tecla(VK_CONTROL)
    tecla(0x12)  # ALT
    tecla(VK_C)
    tecla(VK_C, soltar=True)
    tecla(0x12, soltar=True)
    tecla(VK_CONTROL, soltar=True)


def arrastar(x0, y0, x1, y1):
    user32.SetCursorPos(x0, y0)
    time.sleep(0.15)
    user32.mouse_event(MOUSEEVENTF_LEFTDOWN, 0, 0, 0, None)
    passos = 12
    for i in range(1, passos + 1):
        user32.SetCursorPos(x0 + (x1 - x0) * i // passos, y0 + (y1 - y0) * i // passos)
        time.sleep(0.04)
    user32.mouse_event(MOUSEEVENTF_LEFTUP, 0, 0, 0, None)


def clicar(x, y):
    user32.SetCursorPos(x, y)
    time.sleep(0.2)
    user32.mouse_event(MOUSEEVENTF_LEFTDOWN, 0, 0, 0, None)
    time.sleep(0.08)
    user32.mouse_event(MOUSEEVENTF_LEFTUP, 0, 0, 0, None)


def esperar_menu(segundos=4.0):
    limite = time.time() + segundos
    while time.time() < limite:
        hwnd = user32.FindWindowW("CorretorMenuPopup", None)
        if hwnd and user32.IsWindowVisible(hwnd):
            return hwnd
        time.sleep(0.1)
    return None


def esperar_flyout(segundos=4.0):
    limite = time.time() + segundos
    while time.time() < limite:
        hwnd = user32.FindWindowW("CorretorFlyoutPopup", None)
        if hwnd and user32.IsWindowVisible(hwnd):
            return hwnd
        time.sleep(0.1)
    return None


def esperar_balao(segundos=8.0):
    limite = time.time() + segundos
    while time.time() < limite:
        hwnd = user32.FindWindowW("CorretorBalaoPopup", None)
        if hwnd and user32.IsWindowVisible(hwnd):
            return hwnd
        time.sleep(0.1)
    return None


def executar(hwnd):
    try:
        if not focar(hwnd, EDIT):
            resultados["erro"] = "nao consegui focar a janela de teste"
            return

        r = rect(EDIT)
        antes = edit_read(EDIT)
        resultados["antes"] = antes
        if ATALHO:
            # seleciona tudo e usa o atalho global (o menu deve abrir do mesmo jeito)
            user32.SendMessageW(EDIT, EM_SETSEL, 0, -1)
            time.sleep(0.3)
            atalho_global()
        else:
            # arrasta do fim para o começo da linha de texto
            arrastar(r.left + 700, r.top + 20, r.left + 6, r.top + 20)
        resultados["foco_apos_arrasto"] = user32.GetForegroundWindow() == hwnd

        menu = esperar_menu()
        if not menu:
            resultados["erro"] = "o menu nao apareceu"
            return

        itens = [(filho, texto_janela(filho)) for filho in filhos(menu)]
        resultados["itens"] = [t for _, t in itens]
        resultados["foco_com_menu_aberto"] = user32.GetForegroundWindow() == hwnd

        alvo = next((filho for filho, t in itens if t.lower().startswith(ROTULO_ACAO)), None)
        if alvo is None:
            resultados["erro"] = "botao %s nao encontrado" % ROTULO_ACAO
            return
        rb = rect(alvo)
        resultados["foco_ao_clicar"] = user32.GetForegroundWindow() == hwnd

        if PELA_SETA:
            # clique na seta do botao abre o flyout de variantes
            clicar(rb.right - 8, (rb.top + rb.bottom) // 2)
            flyout = esperar_flyout()
            resultados["flyout_apareceu"] = bool(flyout)
            if not flyout:
                resultados["erro"] = "o flyout do botao nao abriu"
                return
            itens_flyout = [(f, texto_janela(f)) for f in filhos(flyout)]
            resultados["itens_flyout"] = [t for _, t in itens_flyout if t]
            variante = next((f for f, t in itens_flyout if t.startswith(VARIANTE)), None)
            if variante is None:
                resultados["erro"] = "variante %s nao encontrada" % VARIANTE
                return
            if MELHORAR_FORMAL:
                try:
                    open(LOG_MOCK, "w", encoding="utf-8").close()
                except OSError:
                    pass
                antes_log = ""
            else:
                clip_set("SENTINELA-FLYOUT")
                antes_log = None
            rf = rect(variante)
            clicar((rf.left + rf.right) // 2, (rf.top + rf.bottom) // 2)

            if MELHORAR_FORMAL:
                # espera a requisição chegar no mock e o app aplicar o resultado
                limite = time.time() + 25
                while time.time() < limite:
                    try:
                        antes_log = open(LOG_MOCK, encoding="utf-8", errors="replace").read()
                    except OSError:
                        antes_log = ""
                    if antes_log.strip() and edit_read(EDIT) != antes:
                        break
                    time.sleep(0.3)
                resultados["pedido_ia"] = antes_log
            else:
                limite = time.time() + 25
                while time.time() < limite:
                    if clip_get() != "SENTINELA-FLYOUT":
                        break
                    time.sleep(0.3)
                resultados["clipboard"] = clip_get()
            resultados["depois"] = edit_read(EDIT)
            resultados["foco_final"] = user32.GetForegroundWindow() == hwnd
            return

        if SUGESTAO is not None or REVISAR:
            # abre o flyout pela seta do "Corrigir" e usa a lista de avisos
            rc = rect(alvo)
            clicar(rc.right - 8, (rc.top + rc.bottom) // 2)
            flyout = esperar_flyout()
            resultados["flyout_apareceu"] = bool(flyout)
            if not flyout:
                resultados["erro"] = "o flyout do Corrigir nao abriu"
                return
            opcoes = [(f, texto_janela(f)) for f in filhos(flyout)]
            resultados["itens_flyout"] = [t for _, t in opcoes if t]

            if REVISAR:
                item = next((f for f, t in opcoes if t.lower().startswith("revisar")), None)
                if item is None:
                    resultados["erro"] = "item de revisao nao encontrado no flyout"
                    return
                rv = rect(item)
                clicar((rv.left + rv.right) // 2, (rv.top + rv.bottom) // 2)
                limite = time.time() + 25
                janela = None
                while time.time() < limite:
                    janela = user32.FindWindowW(None, "Corretor Ortográfico Universal")
                    if janela and user32.IsWindowVisible(janela):
                        break
                    time.sleep(0.3)
                resultados["janela_do_app_visivel"] = bool(janela) and bool(
                    user32.IsWindowVisible(janela)
                )
                return

            alternativas = [(f, t) for f, t in opcoes if t.startswith("Só isto:")]
            if len(alternativas) < SUGESTAO:
                resultados["erro"] = "esperava %d alternativa(s) na seta, achei %d: %s" % (
                    SUGESTAO, len(alternativas), resultados["itens_flyout"])
                return
            escolhido, rotulo = alternativas[SUGESTAO - 1]
            partes = rotulo.split("→")
            resultados["rotulo_sugestao"] = rotulo
            resultados["trecho"] = partes[0].replace("Só isto:", "").strip() if partes else ""
            resultados["sugestao"] = partes[1].strip() if len(partes) > 1 else ""
            re = rect(escolhido)
            clicar((re.left + re.right) // 2, (re.top + re.bottom) // 2)
            limite = time.time() + 25
            while time.time() < limite:
                if edit_read(EDIT) != antes:
                    break
                time.sleep(0.3)
            resultados["depois"] = edit_read(EDIT)
            resultados["foco_final"] = user32.GetForegroundWindow() == hwnd
            return

        if DICIONARIO:
            alvo_mais = None
            for filho, texto in itens:
                if texto.startswith("Mais ações"):
                    alvo_mais = (filho, texto)
            if alvo_mais is None:
                resultados["erro"] = "botao Mais acoes nao encontrado"
                return
            rm = rect(alvo_mais[0])
            clicar((rm.left + rm.right) // 2, (rm.top + rm.bottom) // 2)
            flyout = esperar_flyout()
            if not flyout:
                resultados["erro"] = "o flyout do Mais acoes nao abriu"
                return
            item = next((f for f in filhos(flyout)
                         if texto_janela(f).startswith("Adicionar seleção")), None)
            if item is None:
                resultados["erro"] = "item do dicionario nao encontrado"
                return
            ri = rect(item)
            clicar((ri.left + ri.right) // 2, (ri.top + ri.bottom) // 2)
            time.sleep(2.0)
            try:
                with open(CAMINHO_CONFIG, encoding="utf-8") as arquivo:
                    resultados["dicionario"] = json.load(arquivo).get("dicionario")
            except OSError as erro:
                resultados["erro"] = "nao consegui ler a config: %s" % erro
                return
            resultados["depois"] = edit_read(EDIT)
            resultados["foco_final"] = user32.GetForegroundWindow() == hwnd
            return

        if COPIAR:
            clip_set("SENTINELA-COPIAR")
            clicar((rb.left + rb.right) // 2, (rb.top + rb.bottom) // 2)
            limite = time.time() + 10
            while time.time() < limite:
                if clip_get() != "SENTINELA-COPIAR":
                    break
                time.sleep(0.2)
            resultados["clipboard"] = clip_get()
            resultados["depois"] = edit_read(EDIT)
            resultados["foco_final"] = user32.GetForegroundWindow() == hwnd
            return

        clicar((rb.left + rb.right) // 2, (rb.top + rb.bottom) // 2)

        if SOMENTE_LEITURA:
            balao = esperar_balao()
            resultados["balao_apareceu"] = bool(balao)
            if balao:
                textos = [t for t in (texto_janela(f) for f in filhos(balao)) if t]
                resultados["balao_texto"] = next((t for t in textos if "programaç" in t), "")
                resultados["balao_botoes"] = [t for t in textos if "Copiar" in t]
                botao = next((f for f in filhos(balao) if texto_janela(f).startswith("Copiar")), None)
                if botao:
                    clip_set("SENTINELA-BALAO")
                    rb2 = rect(botao)
                    clicar((rb2.left + rb2.right) // 2, (rb2.top + rb2.bottom) // 2)
                    time.sleep(1.0)
                    resultados["clipboard_apos_copiar"] = clip_get()
            resultados["depois"] = edit_read(EDIT)
            return

        limite = time.time() + 20
        depois = antes
        while time.time() < limite:
            atual = edit_read(EDIT)
            if atual != antes:
                depois = atual
                break
            time.sleep(0.25)
        resultados["depois"] = depois
        resultados["foco_final"] = user32.GetForegroundWindow() == hwnd
    except Exception:
        import traceback
        resultados["erro"] = traceback.format_exc()
    finally:
        user32.PostMessageW(hwnd, WM_CLOSE, 0, 0)


def main():
    global EDIT, janela
    hinst = kernel32.GetModuleHandleW(None)

    def wndproc(hwnd, msg, wparam, lparam):
        if msg == WM_DESTROY:
            user32.PostQuitMessage(0)
            return 0
        return user32.DefWindowProcW(hwnd, msg, wparam, lparam)

    proc = ctypes.WINFUNCTYPE(wintypes.LPARAM, wintypes.HWND, wintypes.UINT, wintypes.WPARAM, wintypes.LPARAM)(wndproc)
    classe = "ClasseTesteMenuE2E"
    wc = WNDCLASSEXW()
    wc.cbSize = ctypes.sizeof(WNDCLASSEXW)
    wc.lpfnWndProc = ctypes.cast(proc, ctypes.c_void_p)
    wc.hInstance = hinst
    wc.lpszClassName = classe
    if not user32.RegisterClassExW(ctypes.byref(wc)):
        print("FALHA ao registrar a classe")
        return 2

    janela = user32.CreateWindowExW(0, classe, "Janela de teste do menu", 0x00CF0000,
                                    120, 120, 760, 260, None, None, hinst, None)
    if not janela:
        print("FALHA ao criar a janela")
        return 2
    estilo = WS_CHILD | WS_VISIBLE | WS_VSCROLL | WS_TABSTOP | ES_MULTILINE | ES_AUTOVSCROLL
    if SOMENTE_LEITURA:
        estilo |= ES_READONLY
    EDIT = user32.CreateWindowExW(WS_EX_CLIENTEDGE, "EDIT", TEXTO, estilo,
                                  0, 0, 740, 220, janela, None, hinst, None)
    if not EDIT:
        print("FALHA ao criar o EDIT")
        return 2

    user32.ShowWindow(janela, SW_SHOW)
    user32.UpdateWindow(janela)
    time.sleep(0.6)

    threading.Thread(target=executar, args=(janela,), daemon=True).start()
    msg = MSG()
    while user32.GetMessageW(ctypes.byref(msg), None, 0, 0) > 0:
        user32.TranslateMessage(ctypes.byref(msg))
        user32.DispatchMessageW(ctypes.byref(msg))

    print("modo somente-leitura:", SOMENTE_LEITURA, "| atalho:", ATALHO)
    for chave in ["antes", "itens", "foco_apos_arrasto", "foco_com_menu_aberto", "foco_ao_clicar",
                  "depois", "foco_final", "balao_apareceu", "balao_texto", "balao_botoes",
                  "clipboard_apos_copiar", "erro"]:
        if chave in resultados:
            print("%-22s: %r" % (chave, resultados[chave]))

    if resultados.get("erro"):
        print("RESULTADO: FALHA -", resultados["erro"])
        return 3

    if SOMENTE_LEITURA:
        if resultados.get("depois") == resultados.get("antes"):
            print("ok: o campo somente-leitura não foi alterado")
        else:
            print("FALHA: o app alterou um campo somente-leitura")
            return 5
        if "programaç" in (resultados.get("clipboard_apos_copiar") or ""):
            print("RESULTADO: OK - campo não editável: balão com o resultado e botão Copiar")
            return 0
        print("RESULTADO: FALHA - o botão Copiar do balão não copiou o resultado")
        return 5

    # a barra de ações tem 5 botões (o resto das ações fica nos flyouts)
    itens = resultados.get("itens") or []
    esperados = ["Corrigir", "Melhorar", "Copiar", "Mais ações", "Fechar"]
    if itens != esperados:
        print("RESULTADO: FALHA - itens da barra inesperados: %s" % itens)
        return 4

    if SUGESTAO is not None:
        if not resultados.get("flyout_apareceu"):
            print("RESULTADO: FALHA - a seta do Corrigir não abriu")
            return 4
        sugestao = resultados.get("sugestao") or ""
        trecho = resultados.get("trecho") or ""
        depois = resultados.get("depois") or ""
        antes = resultados.get("antes") or ""
        if not sugestao:
            print("RESULTADO: FALHA - não achei a alternativa na seta: %s"
                  % resultados.get("itens_flyout"))
            return 4
        if sugestao not in depois:
            print("RESULTADO: FALHA - a alternativa %r não foi aplicada (%r)" % (sugestao, depois))
            return 4
        if trecho and trecho in depois:
            print("RESULTADO: FALHA - o trecho antigo %r continua no texto" % trecho)
            return 4
        # "só isto": o outro erro do texto tem de continuar intacto
        if " vc " in antes and " vc " not in depois:
            print("RESULTADO: FALHA - outros avisos também foram aplicados (%r)" % depois)
            return 4
        if not resultados.get("foco_final"):
            print("RESULTADO: FALHA - o foco saiu da janela de destino")
            return 4
        print("RESULTADO: OK - seta do Corrigir aplicou só %r (%r → %r)"
              % (sugestao, trecho, sugestao))
        return 0

    if REVISAR:
        if not resultados.get("flyout_apareceu"):
            print("RESULTADO: FALHA - a seta do Corrigir não abriu")
            return 4
        if not resultados.get("janela_do_app_visivel"):
            print("RESULTADO: FALHA - \"Revisar avisos…\" não abriu a janela")
            return 4
        print("RESULTADO: OK - \"Revisar avisos…\" abre a janela com a lista")
        return 0

    if DICIONARIO:
        palavra = (resultados.get("antes") or "").strip().lower()
        if palavra not in (resultados.get("dicionario") or []):
            print("RESULTADO: FALHA - \"%s\" não entrou no dicionário: %s"
                  % (palavra, resultados.get("dicionario")))
            return 4
        if resultados.get("depois") != resultados.get("antes"):
            print("RESULTADO: FALHA - a ação do dicionário mexeu no texto")
            return 4
        if not resultados.get("foco_final"):
            print("RESULTADO: FALHA - o foco saiu da janela de destino")
            return 4
        print("RESULTADO: OK - ⋮ > Adicionar seleção ao dicionário guardou \"%s\"" % palavra)
        return 0

    if COPIAR:
        if (resultados.get("clipboard") or "") != resultados.get("antes"):
            print("RESULTADO: FALHA - o botão Copiar não copiou a seleção: %r"
                  % resultados.get("clipboard"))
            return 4
        if resultados.get("depois") != resultados.get("antes"):
            print("RESULTADO: FALHA - o botão Copiar alterou o campo")
            return 4
        if not resultados.get("foco_final"):
            print("RESULTADO: FALHA - o foco saiu da janela de destino")
            return 4
        print("RESULTADO: OK - botão Copiar copia a seleção sem mexer no campo")
        return 0

    if PELA_SETA:
        if not resultados.get("flyout_apareceu"):
            print("RESULTADO: FALHA - o flyout não abriu")
            return 4
        esperado = VARIANTE.lower()
        if not any(t.lower().startswith(esperado) for t in resultados.get("itens_flyout") or []):
            print("RESULTADO: FALHA - variante ausente: %s" % resultados.get("itens_flyout"))
            return 4
        if MELHORAR_FORMAL:
            pedido = resultados.get("pedido_ia") or ""
            if "tom formal" not in pedido:
                print("RESULTADO: FALHA - a IA não recebeu a instrução de 'Mais formal': %r"
                      % pedido[-200:])
                return 4
            if resultados.get("depois") == resultados.get("antes"):
                print("RESULTADO: FALHA - o texto não foi substituído pelo resultado da IA")
                return 4
            if not resultados.get("foco_final"):
                print("RESULTADO: FALHA - o foco saiu da janela de destino")
                return 4
            print("RESULTADO: OK - 'Melhorar > Mais formal' manda o prompt certo e aplica o resultado")
            return 0

        if "programaç" not in (resultados.get("clipboard") or ""):
            print("RESULTADO: FALHA - o flyout não copiou a correção: %r" % resultados.get("clipboard"))
            return 4
        if resultados.get("depois") != resultados.get("antes"):
            print("RESULTADO: FALHA - o campo foi alterado (deveria ficar intacto): %r"
                  % resultados.get("depois"))
            return 4
        if not resultados.get("foco_final"):
            print("RESULTADO: FALHA - o foco saiu da janela de destino")
            return 4
        print("RESULTADO: OK - flyout do Corrigir copia a correção e deixa o campo intacto")
        return 0
    if not (resultados.get("depois") or "") != resultados.get("antes"):
        print("RESULTADO: FALHA - o texto da janela não mudou")
        return 4
    esperado = "mercado" if MELHORAR else "programação"
    if esperado in (resultados.get("depois") or ""):
        print("RESULTADO: OK - menu + ação '%s' aplicada no texto selecionado" % ROTULO_ACAO)
        return 0
    print("RESULTADO: FALHA - texto mudou, mas sem a correção esperada")
    return 5


if __name__ == "__main__":
    sys.exit(main())
