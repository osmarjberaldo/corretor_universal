# Extensão do Corretor Universal para Google Chrome e Microsoft Edge

Esta extensão integra o **Corretor Ortográfico Universal** diretamente ao **menu de clique direito (menu de contexto)** do seu navegador.

Ao selecionar qualquer texto na web e clicar com o **botão direito**, o menu exibirá as opções do aplicativo:

* **A✓ Corrigir seleção** (LanguageTool + fallback inteligente)
* **🪄 Melhorar com IA (Clareza)**
* **💼 Tornar mais formal**
* **💡 Tornar mais simples**
* **✂️ Encurtar texto**
* **❐ Copiar texto corrigido**
* **💬 Abrir no Balão Flutuante**

---

## 🚀 Como Instalar no Navegador (Passo a Passo)

### No Google Chrome / Brave / Opera:
1. Abra o navegador e digite na barra de endereços: `chrome://extensions`
2. No canto superior direito, ative a chave **"Modo do desenvolvedor"** (*Developer mode*).
3. No canto superior esquerdo, clique no botão **"Carregar sem compactação"** (*Load unpacked*).
4. Selecione a pasta `extension` dentro do diretório do projeto:
   `C:\xampp\htdocs\corretor_universal\extension`
5. Pronto! A extensão **Corretor Universal** estará instalada e ativa.

### No Microsoft Edge:
1. Abra o Edge e acesse: `edge://extensions`
2. Na barra lateral esquerda, ative a opção **"Modo de desenvolvedor"**.
3. Clique em **"Carregar extensão descompactada"**.
4. Selecione a mesma pasta `extension`.

---

## ⚡ Como Funciona a Integração
* **Com o App Desktop aberto**: A extensão conecta-se instantaneamente via API local em `http://127.0.0.1:23456`, utilizando suas configurações personalizadas (chave de IA, prompts e dicionário pessoal), além de salvar tudo no seu histórico.
* **Em campos editáveis** (mensagens, e-mails, redes sociais): O texto é substituído diretamente no campo.
* **Em áreas de leitura** (artigos, notícias, PDFs): O texto corrigido é copiado para a área de transferência e exibido em um balão flutuante na própria página com 1 clique para copiar ou substituir.
