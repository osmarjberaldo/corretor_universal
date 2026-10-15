"""Testa o modo bandeja.

1. fecha a janela principal (WM_CLOSE) e confere que ela some mas o processo
   continua vivo (é o "fica sempre ativo no relógio do Windows");
2. procura o ícone do app na área de notificação (registro do Windows 11);
3. depois disso o motor tem que continuar funcionando — o teste do menu roda
   com a janela escondida (ver tools/e2e_menu.py).
"""
import ctypes
import subprocess
import sys
from ctypes import wintypes

sys.stdout.reconfigure(encoding="utf-8", errors="replace")

user32 = ctypes.windll.user32

TITULO = "Corretor Ortográfico Universal"
WM_CLOSE = 0x0010

user32.FindWindowW.restype = wintypes.HWND
user32.FindWindowW.argtypes = [wintypes.LPCWSTR, wintypes.LPCWSTR]
user32.IsWindowVisible.argtypes = [wintypes.HWND]
user32.PostMessageW.argtypes = [wintypes.HWND, wintypes.UINT, wintypes.WPARAM, wintypes.LPARAM]


def processos(nome):
    saida = subprocess.run(["tasklist", "/FI", "IMAGENAME eq %s" % nome, "/FO", "CSV", "/NH"],
                           capture_output=True, text=True).stdout
    return [linha for linha in saida.splitlines() if nome in linha]


def main():
    hwnd = user32.FindWindowW(None, TITULO)
    if not hwnd:
        print("FALHA: janela do app não encontrada")
        return 2
    if not user32.IsWindowVisible(hwnd):
        print("AVISO: a janela já estava escondida")

    user32.PostMessageW(hwnd, WM_CLOSE, 0, 0)
    import time
    time.sleep(1.5)

    escondida = not user32.IsWindowVisible(hwnd)
    vivos = processos("corretor_universal.exe")
    print("janela escondida após fechar : %s" % escondida)
    print("processo ainda rodando        : %s" % bool(vivos))

    registro = subprocess.run(
        ["reg", "query", r"HKCU\Control Panel\NotifyIconSettings", "/s"],
        capture_output=True, text=True).stdout
    entradas = [linha.strip() for linha in registro.splitlines() if "corretor_universal" in linha.lower()]
    print("ícone na área de notificação  : %s" % (entradas[:2] if entradas else "não encontrado no registro"))

    if escondida and vivos:
        print("RESULTADO: OK - fechar a janela deixa o app ativo na bandeja")
        return 0
    print("RESULTADO: FALHA - o app não ficou ativo em segundo plano")
    return 3


if __name__ == "__main__":
    sys.exit(main())
