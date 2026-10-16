# Corretor Ortográfico Universal (Rust)

> **Projeto 100% Gratuito e de Código Aberto (Open Source - Licença MIT)**  
> 🤖 **Desenvolvido com o auxílio de Inteligência Artificial (IA)**.  
> ✨ Sinta-se livre para usar, estudar, modificar e compartilhar como e onde quiser!  
> 💧 **Novo visual**: barra flutuante translúcida estilo gota d'água / pílula (*Apple Liquid Glass*), com reflexo especular, ícones nítidos e detecção inteligente que dispara **apenas** quando houver texto realmente selecionado.

Selecione um texto (arrastando o mouse ou dando duplo clique numa palavra) e uma **barra de ações aparece sozinha**
no cursor, com acabamento nativo elegante (ícones em vetor, tipografia límpida, separadores sutis,
botão de cópia, menu de varinha mágica para IA, "⋮" e "✕"):

```
[ A ] │ [ A✓ Corrigir ]  [ 🪄 ⌄ ]  [ ❐ ] │ [ ⋮ ]  [ ✕ ]
```

| Botão | O que faz |
| --- | --- |
| **A** | Marca do aplicativo em azul ciano limpo nativo. |
| **A✓ Corrigir** | Ortografia/gramática pelo LanguageTool (com fallback inteligente para IA se a API oscilar). Campo editável → substitui a seleção no lugar; campo travado → balão escuro com o resultado. |
| **🪄 ⌄** | Menu compacto de IA (varinha mágica): *Clareza e gramática*, *Mais formal*, *Mais simples*, *Mais curto*. |
| **❐ Copiar** (só o ícone) | Copia a seleção como está, sem chamar API/IA. |
| **⋮** | Flyout: *Revisar avisos na janela…*, *Histórico de correções (10 recentes)*, *Configurações do app*, *Adicionar seleção ao dicionário*, *Pausar a captura*, *Sair*. |
| **✕** | Fecha a barra (o mesmo que `Esc` ou clicar fora). |

A barra **não rouba o foco**: o clique em "Corrigir" cola no campo onde você
estava, e não no popup (as janelas são criadas com `WS_EX_NOACTIVATE`).

## Novidades e Recursos Recentes

* 🚀 **Inicialização Automática com o Sistema (Windows, macOS e Linux)**:
  * **Windows**: Integração nativa com o Registro (`HKCU\Software\Microsoft\Windows\CurrentVersion\Run`), sem exigir permissões de administrador.
  * **macOS**: Integração automática com LaunchAgents (`~/Library/LaunchAgents/com.corretoruniversal.app.plist`).
  * **Linux**: Padrão oficial XDG Autostart (`~/.config/autostart/corretor_universal.desktop`).
  * Pode ser ativada/desativada tanto pela aba *⚙️ IA & Opções* quanto pelo menu de contexto da bandeja (*Iniciar com o sistema*).
* 🖼️ **Ícone Nativo Unificado de Alta Definição**:
  * O ícone translúcido Apple Aqua / gota d'água agora é renderizado em perfeita resolução (32x32 RGBA) em todos os pontos do sistema: executável `.exe`, bandeja da barra de tarefas (systray) e barra de título da janela do app.
* 🪟 **Design Nativo Compacto & Estável**: Barra em retângulo arredondado moderno (raio de 8px, altura de 34px) com contorno sutil de 1px, sem proporções exageradas de cápsula.
* ✍️ **Ícones Vetoriais Nativos de Alta Resolução**:
  * **A✓**: Símbolo clássico de verificação ortográfica (A com tique subscrito).
  * **🪄 ⌄**: Varinha mágica elegante com centelhas estilo IA e chevron compacto.
  * **❐**: Folhas de documento sobrepostas nativas com alinhamento ortogonal.
  * **⋮**: Três pontos verticais perfeitamente centralizados e dimensionados.
  * **✕**: Cruz fina, nítida e simétrica.
* 📜 **Histórico Persistente (10 recentes)**: Salva automaticamente as últimas 10 correções em `%APPDATA%\corretor_universal\historico.json`. Acompanhe original vs. corrigido, data/hora, tipo de motor (LanguageTool / IA) e copie facilmente.
* 🤖 **Fallback Automático para IA**: Se o serviço público LanguageTool estiver indisponível ou sofrer com limite de requisições, o motor recorre à sua IA configurada de modo transparente.
* 🔄 **Reinício Automático Opcional**: Opção na aba *Configurações* para reiniciar o app automaticamente ao salvar novas credenciais/modelos de IA, recarregando conexões imediatamente.
* 🎯 **Botão de Teste Visual**: Na aba *Teste e Avisos*, utilize o botão **`🎯 Testar Barra Flutuante Agora`** para abrir a barra instantaneamente na posição do mouse e inspecionar os ícones e efeitos de hover.

## Revisão aviso por aviso (como no LanguageTool)

A verificação devolve a **lista de avisos** com as alternativas que a própria API
ofereceu (nada de aplicar às cegas a primeira sugestão de tudo). Na janela do app:

| Controle | O que faz |
| --- | --- |
| **caixa de seleção** de cada aviso | marca/desmarca aquele aviso (desmarcado fica no texto) |
| **alternativas** (botões abaixo do aviso) | escolhe qual sugestão usar — a primeira é a que a API considera melhor |
| **Ignorar** | deixa este aviso fora, sem mexer no dicionário |
| **Dicionário** | protege a palavra (nunca mais corrigida) e desmarca o aviso |
| **Aplicar N aviso(s) e copiar o resultado** | monta o texto só com os avisos marcados e copia (não altera o campo de outro app) |
| **Verificar seleção** | roda de novo a verificação no texto selecionado no outro aplicativo |
| **Marcar todos / Limpar lista** | atalhos para a lista |

Cada aviso mostra também a regra, a categoria e a explicação da API, por exemplo
`MORFOLOGIK_RULE_PT_BR · TYPOS · Encontrado possível erro de ortografia.`

Quando a barra aparece, o app tenta pegar o texto **só pelo UI Automation** (sem
mexer na sua área de transferência) para já oferecer as alternativas na seta.
Sem texto pelo UIA, a lista completa sai pelo "Revisar avisos…" / "Verificar
seleção", que usa a captura normal (UI Automation e, se precisar, `Ctrl+C`).

Quando o campo **não é editável** (página web, PDF, mensagem recebida), nada é
colado: aparece um **balão** escuro com o resultado e o botão **Copiar resultado**
(que copia o resultado já obtido, sem refazer a consulta).

O app também fica **sempre ativo na bandeja** (ícone perto do relógio, menu
"Abrir / Captura ativa / Menu ao selecionar texto / Sair"). Fechar a janela só
esconde — para sair de verdade use **Sair** no menu da bandeja.

O atalho global `Ctrl+Alt+C` (`Cmd+Alt+C` no macOS) continua funcionando e abre
a mesma barra.

## Build e Execução (Todos os Sistemas Operacionais)

### 1. Windows

O projeto no Windows suporta duas abordagens de compilação: **GNU (MinGW-w64)** ou **MSVC (Visual Studio)**.

#### Opção A: Ambiente GNU / MinGW-w64 portátil (sem Visual Studio)
Ideal para compilar sem precisar instalar o Visual Studio completo (~10 GB):

1. **Instalar o Rust com Toolchain GNU**:
   - Baixe o instalador oficial [rustup-init.exe](https://win.rustup.rs/x86_64).
   - Execute no PowerShell para instalar o toolchain GNU minimal:
     ```powershell
     .\rustup-init.exe -y --default-host x86_64-pc-windows-gnu --default-toolchain stable --profile minimal
     ```
2. **MinGW-w64 portátil (`w64devkit`)**:
   - Baixe a versão mais recente do [w64devkit](https://github.com/skeeto/w64devkit/releases).
   - Extraia a pasta `w64devkit` em `%USERPROFILE%\tools\w64devkit\w64devkit`.
3. **Shim do `libgcc_eh.a`**:
   - O w64devkit em 64-bit não distribui `libgcc_eh.a` avulso. Crie a pasta `%USERPROFILE%\tools\mingw-rust-shim` e gere o shim vazio:
     ```powershell
     New-Item -ItemType Directory -Path "$env:USERPROFILE\tools\mingw-rust-shim" -Force
     & "$env:USERPROFILE\tools\w64devkit\w64devkit\bin\x86_64-w64-mingw32-ar.exe" crs "$env:USERPROFILE\tools\mingw-rust-shim\libgcc_eh.a"
     ```
4. **Configuração do `.cargo/config.toml`**:
   - Mantenha o arquivo `.cargo/config.toml` com:
     ```toml
     [target.x86_64-pc-windows-gnu]
     linker = "x86_64-w64-mingw32-gcc"
     rustflags = ["-L", "C:\\Users\\<SEU_USUARIO>\\tools\\mingw-rust-shim"]
     ```
5. **Compilar (escolha uma das opções)**:
   - No **PowerShell**: `.\build.ps1`
   - No **Prompt de Comando / Duplo clique no Explorer**: `build.bat`
   - No **Git Bash**: `./build.sh`
   - Ou diretamente com Cargo:
     ```powershell
     $env:PATH = "$env:USERPROFILE\tools\w64devkit\w64devkit\bin;$env:USERPROFILE\.cargo\bin;$env:PATH"
     cargo build --release
     ```
6. **Executar**:
   ```powershell
   .\target\release\corretor_universal.exe
   ```

#### Opção B: MSVC (Visual Studio Build Tools)
Se você já possui o Visual Studio ou os Build Tools com C++ instalados:

1. **Configurar o toolchain MSVC**:
   ```powershell
   rustup default stable-x86_64-pc-windows-msvc
   ```
2. **Remover a personalização GNU**:
   - Exclua ou renomeie o arquivo `.cargo/config.toml`.
3. **Compilar**:
   ```powershell
   cargo build --release
   ```
4. **Executar**:
   ```powershell
   .\target\release\corretor_universal.exe
   ```

---

### 2. Linux (Ubuntu, Debian, Fedora, Arch)

No Linux, o app utiliza X11/OpenGL e a API pública do LanguageTool. A detecção automática de cliques e UIA é específica do Windows; no Linux a interface abre e responde ao atalho global **`Ctrl + Alt + C`**.

#### Passo 1: Instalar Dependências de Sistema

- **Debian / Ubuntu / Linux Mint**:
  ```bash
  sudo apt update
  sudo apt install -y build-essential pkg-config libssl-dev \
      libx11-dev libxext-dev libxft-dev libxinerama-dev \
      libxcursor-dev libxi-dev libxrandr-dev libxss-dev libgl1-mesa-dev
  ```

- **Fedora / RHEL**:
  ```bash
  sudo dnf install -y gcc pkgconf-pkg-config openssl-devel \
      libX11-devel libXcursor-devel libXrandr-devel libXi-devel mesa-libGL-devel
  ```

- **Arch Linux / Manjaro**:
  ```bash
  sudo pacman -S --needed base-devel openssl libx11 libxcursor libxrandr libxi mesa pkgconf
  ```

#### Passo 2: Instalar o Rust
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

#### Passo 3: Compilar e Rodar
Certifique-se de que o arquivo `.cargo/config.toml` (específico para Windows GNU) não interfira no build do Linux. O Cargo ignora o alvo `x86_64-pc-windows-gnu` automaticamente no Linux.

```bash
cargo build --release
./target/release/corretor_universal
```

> **Dica Wayland**: Se utilizar ambientes como GNOME/KDE sob Wayland nativo e o atalho global não disparar, execute com XWayland ou configure o atalho de teclado global do próprio ambiente chamando `./target/release/corretor_universal`.

---

### 3. macOS (Apple Silicon M1/M2/M3/M4 & Intel)

No macOS, o atalho global padrão é **`Cmd + Option + C`** (ou `Ctrl + Alt + C`).

#### Passo 1: Instalar Xcode Command Line Tools
Abra o Terminal e execute:
```bash
xcode-select --install
```

#### Passo 2: Instalar o Rust
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

#### Passo 3: Compilar
```bash
cargo build --release
```

#### Passo 4: Conceder Permissões de Acessibilidade
Para que o atalho global funcione entre aplicativos no macOS:
1. Abra **Ajustes do Sistema** (System Settings).
2. Vá em **Privacidade e Segurança > Acessibilidade** (Privacy & Security > Accessibility).
3. Habilite o **Terminal** (ou o executável compilado).

#### Passo 5: Executar
```bash
./target/release/corretor_universal
```

---

## Modos de linha de comando (verificação)

```bash
# correção (LanguageTool): texto, idioma, palavras ignoradas
./target/release/corretor_universal.exe --self-test "Eu gosto de programaçao." pt-BR "rust"

# IA personalizada, opcionalmente sobrescrevendo a configuração
./target/release/corretor_universal.exe --ia-test "texto" --url http://127.0.0.1:8137/v1 \
    --chave K --modelo M

# o que o app enxerga no campo com foco (diagnóstico da detecção automática)
./target/release/corretor_universal.exe --uia-test
```

## Configuração

`%APPDATA%\corretor_universal\config.json` (macOS:
`~/Library/Application Support/corretor_universal/`, Linux:
`~/.config/corretor_universal/`):

```json
{
  "idioma": "pt-BR",
  "ativo": true,
  "deteccao_automatica": true,
  "iniciar_com_sistema": true,
  "dicionario": ["rust"],
  "ia_url": "https://api.openai.com/v1",
  "ia_chave": "sua-chave",
  "ia_modelo": "gpt-4o-mini",
  "ia_prompt": "Reescreva o texto em português do Brasil..."
}
```

`ia_url` aceita a base (`.../v1`) ou a URL completa (`.../v1/chat/completions`).
Qualquer provedor compatível com OpenAI serve (OpenAI, Groq, Gemini via
endpoint compatível, OpenRouter, um modelo local com Ollama/LM Studio etc.).
A chave fica **em texto puro** no arquivo — cuide das permissões dele.

## Verificação feita neste projeto

Harnesses em `tools/` (com o app aberto e o Windows desbloqueado; não use o
mouse/teclado durante o teste):

| Comando | Verifica |
| --- | --- |
| `python tools/e2e_menu.py` | arrasta o mouse para selecionar → barra aparece com os 5 botões → clique em **Corrigir** → seleção substituída, foco nunca sai da janela |
| `python tools/e2e_menu.py --atalho` | o mesmo pelo atalho `Ctrl+Alt+C` |
| `python tools/e2e_menu.py --pela-seta` | seta do **Corrigir** abre o flyout → *Apenas copiar a correção* → corrige sem mexer no campo |
| `python tools/e2e_menu.py --copiar` | botão **⧉** copia a seleção exatamente como está |
| `python tools/e2e_menu.py "palavra" --dicionario` | **⋮ → Adicionar seleção ao dicionário**: a palavra entra no `config.json` e o texto não é tocado (restaure a configuração depois) |
| `python tools/e2e_menu.py --melhorar` | **Melhorar com IA** (use o mock abaixo para não gastar créditos) |
| `python tools/e2e_menu.py --melhorar-formal` | **Melhorar ▾ → Mais formal**: confere no log do mock que a IA recebeu o prompt da variante |
| `python tools/e2e_menu.py --sugestao=N` | a seta do **Corrigir** lista as alternativas reais e aplicar a N-ésima corrige **só** aquele trecho (o outro erro do texto tem de continuar lá) |
| `python tools/e2e_menu.py --revisar` | **Corrigir ▾ → Revisar avisos…** abre a janela com a lista |
| `corretor_universal.exe --analise-test "texto"` | imprime a análise (avisos, alternativas, regra, categoria, mensagem) sem abrir a janela |
| `python tools/e2e_menu.py --somente-leitura` | campo não editável: balão com resultado, botão **Copiar resultado** funcionando, campo intacto |
| `python tools/e2e_barra.py [--pausar]` | estrutura da barra (5 botões na mesma linha), flyout do Corrigir, flyout do ⋮, o **✕** fecha; `--pausar` testa *⋮ → Pausar a captura* de verdade |
| `python tools/e2e_barra.py --acao abrir` | **⋮ → Abrir o app** mostra a janela principal |
| `python tools/e2e_barra.py --acao sair` | **⋮ → Sair** encerra o processo (rode por último) |
| `python tools/captura_barra.py` | fotografa a barra (salva `tools/barra_exemplo.png`) e confere no pixel: fundo escuro `RGB(30,30,32)`, marca em azul, texto claro, cantos arredondados |
| `python tools/e2e_tray.py` | fechar a janela → continua ativo na bandeja (ícone no registro da área de notificação) |

`e2e_barra.py` e `captura_barra.py` **não** precisam que a janela de destino
esteja em primeiro plano (usam o atalho global), então servem para conferir a
barra mesmo com outra janela na frente.

Resultados medidos nesta máquina: substituição no lugar em **~1,4 s**; barra de
**477x40 px** com os 5 botões; o menu abre sem tirar o foco (a janela de destino
segue em primeiro plano depois do clique no botão); com a janela escondida na
bandeja o motor continua funcionando (mesmo teste de menu passa).

Um mock compatível com OpenAI para testar a IA sem chave real:

```bash
python tools/mock_ia.py 8137            # responde em http://127.0.0.1:8137/v1
```

## Limitações conhecidas

- **Detecção automática só no Windows** (usa UI Automation + mouse nativo). No
  macOS/Linux o atalho global funciona, mas não há barra nativa: a ação é
  executada direto e o resultado aparece na janela do app.
- As **ações extras** (setas/flyouts, ⋮, ✕) existem só na barra; no macOS/Linux,
  que não têm barra nativa, o atalho faz apenas a correção direta.
- **"Adicionar seleção ao dicionário"** aceita só **uma palavra** por vez
  (o dicionário protege palavras soltas; um trecho inteiro não teria sentido).
- Detecta seleção feita com o **mouse** (arrasto ou clique duplo). Seleção por
  teclado (`Shift+setas`, `Ctrl+A`) não abre o menu — use o atalho.
- **Falsos positivos**: `Rust` continua sendo marcado como erro de ortografia e o
  botão "Corrigir" aplica **todos os avisos marcados** (inclusive de estilo), então
  `Rust` ainda vira `Rush` nesse clique. Agora dá para resolver de três formas:
  desmarcar o aviso na lista, clicar em **Dicionário** (nunca mais corrigir a
  palavra) ou usar a seta (**Só isto: …**) para aplicar apenas o que interessa.
- A análise automática ao selecionar usa **só UI Automation**; em campos que não
  expõem texto por UIA a seta mostra *Abrir a lista de avisos…* e a verificação
  sai quando você clica (aí sim com `Ctrl+C`).
- A API pública do LanguageTool limita a **20 KB por requisição, 20 req/min** e
  pede para não automatizar requisições em massa; textos maiores voltam HTTP 413 e
  ainda não são fatiados por frases.
- **Clipboard**: depois de aplicar no campo, a área de transferência fica com o
  texto final (o conteúdo anterior não é restaurado). No caminho do balão o
  conteúdo anterior é preservado até você clicar em "Copiar resultado".
- Os flyouts são janelas próprias (mesmo tema escuro da barra), abertos pela
  seta/"⋮"; eles fecham com `Esc`, clique fora, `✕` ou depois de 12 s.
- **Falha da API sem retry**: se a requisição ao LanguageTool falhar (timeout de
  6 s, HTTP 429), aparece no histórico e nada é aplicado.
- **Campos que não expõem UI Automation** (alguns apps legados, telas
  desenhadas à mão) não acionam o menu nem a checagem de editabilidade — nesses
  casos o atalho ainda tenta corrigir e colar.
- Uso de memória: ~75–135 MB de working set (egui/glow + OpenGL), bem acima dos
  15 MB citados no projeto original.
