"""Abre a barra de ações pelo atalho e tira uma foto dela (tools/barra_exemplo.png).

Além de salvar a imagem, confere no pixel o que o "desenho à mão" deve produzir:

* fundo escuro (a barra não é a janela cinza padrão do Windows);
* a marca "A" em azul (cor de destaque) na primeira área;
* texto claro (quase branco) nos botões;
* cantos arredondados (o pixel do canto não é a cor de fundo da barra).

Uso: python tools/captura_barra.py        (com o app aberto)
"""
import ctypes
import sys
import time
from ctypes import wintypes

sys.stdout.reconfigure(encoding="utf-8", errors="replace")

from PIL import Image, ImageGrab

user32 = ctypes.windll.user32
user32.FindWindowW.restype = wintypes.HWND
user32.FindWindowW.argtypes = [wintypes.LPCWSTR, wintypes.LPCWSTR]
user32.IsWindowVisible.argtypes = [wintypes.HWND]

CLASSE_BARRA = "CorretorMenuPopup"
ARQUIVO = "tools/barra_exemplo.png"
FUNDO = (30, 30, 32)
HOVER = (56, 57, 59)
ACENTO = (88, 166, 255)


class KEYBDINPUT(ctypes.Structure):
    _fields_ = [("wVk", wintypes.WORD), ("wScan", wintypes.WORD), ("dwFlags", wintypes.DWORD),
                ("time", wintypes.DWORD), ("dwExtraInfo", ctypes.c_void_p)]


class _ENTRADA(ctypes.Union):
    _fields_ = [("ki", KEYBDINPUT), ("padding", ctypes.c_byte * 32)]


class INPUT(ctypes.Structure):
    _fields_ = [("type", wintypes.DWORD), ("u", _ENTRADA)]


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


def perto(cor, alvo, tolerancia=12):
    return all(abs(a - b) <= tolerancia for a, b in zip(cor, alvo))


def main():
    atalho_global()
    barra = None
    limite = time.time() + 4
    while time.time() < limite:
        hwnd = user32.FindWindowW(CLASSE_BARRA, None)
        if hwnd and user32.IsWindowVisible(hwnd):
            barra = hwnd
            break
        time.sleep(0.1)
    if not barra:
        print("FALHA: a barra não apareceu (o atalho Ctrl+Alt+C está funcionando?)")
        return 2

    r = wintypes.RECT()
    user32.GetWindowRect(barra, ctypes.byref(r))
    time.sleep(0.4)
    foto = ImageGrab.grab(bbox=(r.left, r.top, r.right, r.bottom)).convert("RGB")
    foto.save(ARQUIVO)
    largura, altura = foto.size
    pixels = foto.load()

    centro = pixels[largura // 2, altura // 4]
    canto = pixels[1, 1]

    # marca (primeiros ~34 px) deve ter pixels de destaque
    destaque = 0
    for x in range(0, min(40, largura)):
        for y in range(altura):
            if perto(pixels[x, y], ACENTO, 40):
                destaque += 1

    # texto claro no restante da barra
    claros = 0
    for x in range(40, largura):
        for y in range(altura):
            p = pixels[x, y]
            if p[0] > 200 and p[1] > 200 and p[2] > 200:
                claros += 1

    print("imagem salva      : %s (%dx%d)" % (ARQUIVO, largura, altura))
    print("pixel do miolo    : %s (esperado ~%s ou ~%s)" % (centro, FUNDO, HOVER))
    print("pixel do canto    : %s (esperado fora da área da barra: cantos arredondados)" % (canto,))
    print("pixels de destaque: %d (marca \"A\")" % destaque)
    print("pixels claros     : %d (texto dos botões)" % claros)

    if not (perto(centro, FUNDO, 10) or perto(centro, HOVER, 10)):
        print("FALHA: o fundo da barra não está escuro (veio %s)" % (centro,))
        return 3
    if destaque < 20:
        print("FALHA: não achei a marca em azul")
        return 3
    if claros < 30:
        print("FALHA: não achei texto claro na barra")
        return 3
    if perto(canto, centro, 3):
        print("AVISO: o canto tem a mesma cor do miolo (a janela pode não estar recortada em canto arredondado;")
        print("       confira na imagem se o fundo atrás era muito parecido com %s)" % (centro,))
    else:
        print("ok: canto diferente do miolo (recorte arredondado)")

    print("RESULTADO: OK - barra escura com marca em destaque e texto claro")
    return 0


if __name__ == "__main__":
    sys.exit(main())
