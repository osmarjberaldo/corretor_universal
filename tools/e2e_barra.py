"""Testa a barra de ações (o "menu" que aparece ao selecionar texto).

Diferente do e2e_menu.py, este teste **não** precisa que a janela de destino
esteja em primeiro plano: ele abre a barra pelo atalho global (Ctrl+Alt+C), o
que também funciona com outro app na frente. Verifica:

1. a barra aparece e tem os 5 botões esperados, todos na mesma linha
   (Corrigir | Melhorar | Copiar | Mais ações | Fechar);
2. a seta do "Corrigir" abre o flyout com as variantes;
3. o "⋮" abre o flyout de opções do app;
4. o "✕" fecha a barra.

Uso: python tools/e2e_barra.py        (com o app aberto)
"""
import ctypes
import subprocess
import sys
import time
from ctypes import wintypes

sys.stdout.reconfigure(encoding="utf-8", errors="replace")

user32 = ctypes.windll.user32
kernel32 = ctypes.windll.kernel32

CLASSE_BARRA = "CorretorMenuPopup"
CLASSE_FLYOUT = "CorretorFlyoutPopup"
CLASSE_BALAO = "CorretorBalaoPopup"
TITULO_EXE = "corretor_universal.exe"
VK_CONTROL = 0x11
VK_MENU = 0x12  # Alt
VK_C = 0x43

user32.FindWindowW.restype = wintypes.HWND
user32.FindWindowW.argtypes = [wintypes.LPCWSTR, wintypes.LPCWSTR]
user32.IsWindowVisible.argtypes = [wintypes.HWND]

PAUSAR = "--pausar" in sys.argv
# --acao abrir | sair : testa um item do flyout "⋮"
ACAO = None
if "--acao" in sys.argv:
    ACAO = sys.argv[sys.argv.index("--acao") + 1]
ROTULO_ACAO = {"abrir": "Abrir o app", "sair": "Sair"}.get(ACAO or "", "")
TITULO_APP = "Corretor Ortográfico Universal"

esperados_barra = ["Corrigir", "Melhorar", "Copiar", "Mais ações", "Fechar"]
esperados_corrigir = ["Substituir a seleção", "Apenas copiar a correção"]
esperados_mais = ["Abrir o app", "Adicionar seleção ao dicionário", "Pausar a captura", "Sair"]

resultados = {}


# --------------------------------------------------------------------------
# entrada (SendInput, que é o que este Windows respeita)
# --------------------------------------------------------------------------
class KEYBDINPUT(ctypes.Structure):
    _fields_ = [("wVk", wintypes.WORD), ("wScan", wintypes.WORD), ("dwFlags", wintypes.DWORD),
                ("time", wintypes.DWORD), ("dwExtraInfo", ctypes.c_void_p)]


class _ENTRADA(ctypes.Union):
    _fields_ = [("ki", KEYBDINPUT), ("padding", ctypes.c_byte * 32)]


class INPUT(ctypes.Structure):
    _fields_ = [("type", wintypes.DWORD), ("u", _ENTRADA)]


user32.SendInput.restype = wintypes.UINT
user32.SendInput.argtypes = [wintypes.UINT, ctypes.POINTER(INPUT), ctypes.c_int]


def _tecla(vk, soltar=False):
    entrada = INPUT()
    entrada.type = 1  # INPUT_KEYBOARD
    entrada.u.ki = KEYBDINPUT(wVk=vk, wScan=0, dwFlags=2 if soltar else 0, time=0, dwExtraInfo=None)
    user32.SendInput(1, ctypes.byref(entrada), ctypes.sizeof(INPUT))
    time.sleep(0.03)


def atalho_global():
    _tecla(VK_CONTROL)
    _tecla(VK_MENU)  # ALT
    _tecla(VK_C)
    _tecla(VK_C, True)
    _tecla(VK_MENU, True)
    _tecla(VK_CONTROL, True)


def clicar(x, y):
    user32.SetCursorPos(x, y)
    time.sleep(0.15)
    user32.mouse_event(0x0002, 0, 0, 0, 0)  # LEFTDOWN
    time.sleep(0.05)
    user32.mouse_event(0x0004, 0, 0, 0, 0)  # LEFTUP
    time.sleep(0.35)


# --------------------------------------------------------------------------
# consultas à janela
# --------------------------------------------------------------------------
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


def processos(nome):
    out = subprocess.run(["tasklist", "/FI", "IMAGENAME eq %s" % nome, "/FO", "CSV", "/NH"],
                         capture_output=True, text=True).stdout
    return [linha for linha in out.splitlines() if nome in linha]


def esperar(classe, segundos=4.0):
    limite = time.time() + segundos
    while time.time() < limite:
        hwnd = user32.FindWindowW(classe, None)
        if hwnd and user32.IsWindowVisible(hwnd):
            return hwnd
        time.sleep(0.1)
    return None


def itens(janela):
    """[(hwnd, texto, rect)] dos botões, na ordem da esquerda para a direita."""
    lista = [(f, texto_janela(f), rect(f)) for f in filhos(janela)]
    lista = [(f, t, r) for f, t, r in lista if t]
    lista.sort(key=lambda item: item[2].left)
    return lista


def botao(janela, prefixo):
    for f, t, r in itens(janela):
        if t.lower().startswith(prefixo.lower()):
            return f, t, r
    return None, None, None


def main():
    hwnd_barra = user32.FindWindowW(CLASSE_BARRA, None)
    if hwnd_barra and user32.IsWindowVisible(hwnd_barra):
        print("AVISO: já existia uma barra aberta; usando o atalho de novo")

    # 1) abre a barra pelo atalho global (não depende do foco)
    atalho_global()
    barra = esperar(CLASSE_BARRA)
    if not barra:
        print("FALHA: a barra não apareceu com o atalho Ctrl+Alt+C")
        return 2

    lista = itens(barra)
    resultados["itens_barra"] = [t for _, t, _ in lista]
    rb = rect(barra)
    resultados["barra"] = {"largura": rb.right - rb.left, "altura": rb.bottom - rb.top}

    faltando = [t for t in esperados_barra if not botao(barra, t)[0]]
    if faltando:
        print("FALHA: faltam botões na barra: %s" % faltando)
        print("       encontrados: %s" % resultados["itens_barra"])
        return 3
    if len(lista) != len(esperados_barra):
        print("FALHA: esperava %d botões, achei %d: %s"
              % (len(esperados_barra), len(lista), resultados["itens_barra"]))
        return 3

    # todos na mesma linha (barra horizontal, não lista vertical)
    topos = {r.top for _, _, r in lista}
    alturas = {r.bottom - r.top for _, _, r in lista}
    resultados["mesma_linha"] = (len(topos) == 1, len(alturas) == 1)
    if len(topos) != 1:
        print("FALHA: os botões não estão na mesma linha (topos %s)" % sorted(topos))
        return 3
    if rb.right - rb.left < rb.bottom - rb.top:
        print("FALHA: a barra saiu vertical (%dx%d)"
              % (rb.right - rb.left, rb.bottom - rb.top))
        return 3
    print("ok: barra horizontal com %d botões: %s" % (len(lista), resultados["itens_barra"]))

    # 2) seta do "Corrigir" abre o flyout das variantes
    _, _, rc = botao(barra, "Corrigir")
    clicar(rc.right - 8, (rc.top + rc.bottom) // 2)
    flyout = esperar(CLASSE_FLYOUT, 3.0)
    if not flyout:
        print("FALHA: a seta do Corrigir não abriu o flyout")
        return 4
    lista_flyout = [t for _, t, _ in itens(flyout)]
    resultados["flyout_corrigir"] = lista_flyout
    if lista_flyout != esperados_corrigir:
        print("FALHA: itens do flyout do Corrigir: %s" % lista_flyout)
        return 4
    print("ok: seta do Corrigir abre: %s" % lista_flyout)

    # fecha o flyout sem executar nada (ESC)
    _tecla(0x1B)  # ESC
    _tecla(0x1B, True)
    time.sleep(0.4)
    if user32.IsWindowVisible(flyout) or user32.IsWindowVisible(barra):
        print("FALHA: ESC não fechou a barra/flyout")
        return 5

    # 3) "⋮" abre o flyout de opções
    atalho_global()
    barra = esperar(CLASSE_BARRA)
    if not barra:
        print("FALHA: a barra não reapareceu")
        return 6
    _, _, rmais = botao(barra, "Mais ações")
    clicar((rmais.left + rmais.right) // 2, (rmais.top + rmais.bottom) // 2)
    flyout = esperar(CLASSE_FLYOUT, 3.0)
    if not flyout:
        print("FALHA: o \"⋮\" não abriu o flyout de opções")
        return 6
    lista_mais = [t for _, t, _ in itens(flyout)]
    resultados["flyout_mais"] = lista_mais
    if lista_mais != esperados_mais:
        print("FALHA: itens do flyout do ⋮: %s" % lista_mais)
        return 6
    print("ok: ⋮ abre: %s" % lista_mais)

    # 4) "✕" fecha tudo
    _, _, rx = botao(barra, "Fechar")
    clicar((rx.left + rx.right) // 2, (rx.top + rx.bottom) // 2)
    time.sleep(0.5)
    barra_depois = user32.IsWindowVisible(barra)
    flyout_depois = user32.IsWindowVisible(flyout)
    resultados["fechou"] = not barra_depois and not flyout_depois
    if barra_depois or flyout_depois:
        print("FALHA: o ✕ não fechou (barra=%s flyout=%s)" % (barra_depois, flyout_depois))
        return 7
    print("ok: o ✕ fecha a barra e o flyout")

    # 5) opcional: "⋮ > Pausar a captura" desliga de verdade (o atalho para de abrir)
    if PAUSAR:
        atalho_global()
        barra = esperar(CLASSE_BARRA)
        if not barra:
            print("FALHA: a barra não abriu para o teste de pausa")
            return 8
        _, _, rmais = botao(barra, "Mais ações")
        clicar((rmais.left + rmais.right) // 2, (rmais.top + rmais.bottom) // 2)
        flyout = esperar(CLASSE_FLYOUT, 3.0)
        if not flyout:
            print("FALHA: o ⋮ não abriu no teste de pausa")
            return 8
        _, _, rpausar = botao(flyout, "Pausar a captura")
        if not rpausar:
            print("FALHA: não achei \"Pausar a captura\"")
            return 8
        clicar((rpausar.left + rpausar.right) // 2, (rpausar.top + rpausar.bottom) // 2)
        time.sleep(1.2)
        atalho_global()
        time.sleep(1.0)
        abriu = bool(user32.FindWindowW(CLASSE_BARRA, None)) and user32.IsWindowVisible(
            user32.FindWindowW(CLASSE_BARRA, None)
        )
        resultados["pausou"] = not abriu
        if abriu:
            print("FALHA: mesmo pausado o atalho abriu a barra")
            return 8
        print("ok: \"Pausar a captura\" desliga a captura (o atalho não abre mais a barra)")
        print("    ATENÇÃO: o app ficou pausado; reative no menu da bandeja"
              " (ou restaure \"ativo\": true na configuração).")

    # 6) opcional: um item do flyout "⋮" (abrir o app / sair)
    if ROTULO_ACAO:
        atalho_global()
        barra = esperar(CLASSE_BARRA)
        if not barra:
            print("FALHA: a barra não abriu para testar \"%s\"" % ROTULO_ACAO)
            return 9
        _, _, rmais = botao(barra, "Mais ações")
        clicar((rmais.left + rmais.right) // 2, (rmais.top + rmais.bottom) // 2)
        flyout = esperar(CLASSE_FLYOUT, 3.0)
        if not flyout:
            print("FALHA: o ⋮ não abriu")
            return 9
        _, _, ritem = botao(flyout, ROTULO_ACAO)
        if not ritem:
            print("FALHA: não achei \"%s\" no flyout" % ROTULO_ACAO)
            return 9
        clicar((ritem.left + ritem.right) // 2, (ritem.top + ritem.bottom) // 2)
        time.sleep(1.5)

        if ACAO == "abrir":
            janela = user32.FindWindowW(None, TITULO_APP)
            visivel = bool(janela) and bool(user32.IsWindowVisible(janela))
            resultados["janela_do_app_visivel"] = visivel
            if not visivel:
                print("FALHA: \"Abrir o app\" não mostrou a janela")
                return 9
            print("ok: \"⋮ > Abrir o app\" mostra a janela do aplicativo")
        else:
            restantes = [l for l in processos(TITULO_EXE) if l]
            resultados["app_encerrado"] = not restantes
            if restantes:
                print("FALHA: \"Sair\" não encerrou o app (%s)" % restantes)
                return 9
            print("ok: \"⋮ > Sair\" encerra o aplicativo (o processo sumiu)")

    print()
    for chave, valor in resultados.items():
        print("%-18s: %r" % (chave, valor))
    print("RESULTADO: OK - barra de ações, setas, ⋮ e ✕ funcionando")
    return 0


if __name__ == "__main__":
    sys.exit(main())
