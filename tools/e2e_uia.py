"""Mostra o que o app enxerga no campo com foco (usa --uia-test).

Cria a mesma janela alvo do e2e_confirmacao.py, seleciona o texto, e roda o
modo --uia-test (binário debug, que tem console) capturando a saída.

Uso: python tools/e2e_uia.py
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

WNDPROC = ctypes.WINFUNCTYPE(ctypes.c_ssize_t, wintypes.HWND, ctypes.c_uint, wintypes.WPARAM, wintypes.LPARAM)
_referencias = []

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

TEXTO = "Eu gosto de programaçao em Pyton ."


class WNDCLASSW(ctypes.Structure):
    _fields_ = [("style", ctypes.c_uint), ("lpfnWndProc", WNDPROC),
                ("cbClsExtra", ctypes.c_int), ("cbWndExtra", ctypes.c_int),
                ("hInstance", wintypes.HINSTANCE), ("hIcon", wintypes.HICON),
                ("hCursor", wintypes.HANDLE), ("hbrBackground", wintypes.HBRUSH),
                ("lpszMenuName", wintypes.LPCWSTR), ("lpszClassName", wintypes.LPCWSTR)]


def criar(proc_ref):
    hinst = kernel32.GetModuleHandleW(None)
    proc = WNDPROC(lambda h, m, w, l: user32.DefWindowProcW(h, m, w, l))
    proc_ref.append(proc)
    wc = WNDCLASSW()
    wc.lpfnWndProc = proc
    wc.hInstance = hinst
    wc.lpszClassName = "CorretorTesteHostUia"
    user32.RegisterClassW(ctypes.byref(wc))
    hwnd = user32.CreateWindowExW(0, "CorretorTesteHostUia", "Alvo do Corretor (uia)",
                                  WS_OVERLAPPEDWINDOW, 300, 220, 620, 240, None, None, hinst, None)
    edit = user32.CreateWindowExW(0, "EDIT", TEXTO,
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


def main():
    exe = sys.argv[1] if len(sys.argv) > 1 else r"target\debug\corretor_universal.exe"
    hwnd, edit = criar(_referencias)
    time.sleep(0.8)
    user32.SetFocus(edit)
    user32.SendMessageW(edit, EM_SETSEL, 0, -1)

    msg = wintypes.MSG()
    resultado = {}

    def roda():
        time.sleep(0.5)
        p = subprocess.run([exe, "--uia-test"], capture_output=True, text=True, timeout=60)
        resultado["out"] = p.stdout
        resultado["err"] = p.stderr
        resultado["code"] = p.returncode

    t = threading.Thread(target=roda, daemon=True)
    t.start()
    limite = time.time() + 65
    while t.is_alive() and time.time() < limite:
        while user32.PeekMessageW(ctypes.byref(msg), None, 0, 0, 1):
            user32.TranslateMessage(ctypes.byref(msg))
            user32.DispatchMessageW(ctypes.byref(msg))
        time.sleep(0.02)

    print("exit:", resultado.get("code"))
    print("--- stdout ---")
    print(resultado.get("out", ""))
    print("--- stderr ---")
    print(resultado.get("err", ""))
    return 0


if __name__ == "__main__":
    sys.exit(main())
