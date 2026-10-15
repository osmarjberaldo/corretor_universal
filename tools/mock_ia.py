"""Servidor mock compatível com a API da OpenAI (/v1/chat/completions).

Usado para verificar a "IA personalizada" do corretor sem depender de serviço
externo nem de chave real. Grava cada requisição recebida em mock_requests.log.
"""
import json
import sys
from http.server import BaseHTTPRequestHandler, HTTPServer

import os
import tempfile
LOG = os.path.join(tempfile.gettempdir(), "mock_ia_requests.log")


class Handler(BaseHTTPRequestHandler):
    def do_POST(self):
        tamanho = int(self.headers.get("Content-Length", 0))
        corpo = self.rfile.read(tamanho).decode("utf-8", "replace")
        with open(LOG, "a", encoding="utf-8") as f:
            f.write("=== %s %s\n" % (self.command, self.path))
            for chave, valor in self.headers.items():
                f.write("H %s: %s\n" % (chave, valor))
            f.write("B %s\n" % corpo)

        resposta = {
            "id": "mock-1",
            "object": "chat.completion",
            "choices": [
                {
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": "Hoje eu fui ao mercado e comprei maçã.",
                    },
                    "finish_reason": "stop",
                }
            ],
        }
        dados = json.dumps(resposta).encode("utf-8")
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(dados)))
        self.end_headers()
        self.wfile.write(dados)

    def log_message(self, *args):
        pass


if __name__ == "__main__":
    porta = int(sys.argv[1]) if len(sys.argv) > 1 else 8137
    print("mock ouvindo na porta %d (log: %s)" % (porta, LOG), flush=True)
    HTTPServer(("127.0.0.1", porta), Handler).serve_forever()
