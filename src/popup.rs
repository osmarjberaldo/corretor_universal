//! Janelas de popup nativas, desenhadas à mão.
//!
//! A **barra de ações** é o menu que aparece ao selecionar um texto: marca,
//! "Corrigir" e "Melhorar" (cada um com a seta que abre um flyout de variantes),
//! "Copiar" (só o ícone), "⋮" (mais opções) e "✕". O **balão** mostra o resultado
//! quando o campo não aceita edição, com o botão "Copiar resultado".
//!
//! No Windows são janelas Win32 criadas com `WS_EX_NOACTIVATE`: aparecem no
//! cursor **sem tirar o foco** do aplicativo de destino. Isso é essencial, porque
//! o `Ctrl+V` da correção precisa chegar no campo onde o usuário estava
//! digitando — e não no popup.
//!
//! Nos outros sistemas o módulo é um stub: o app usa a própria janela do egui
//! para mostrar o resultado (ver `main.rs`).

// ---------------------------------------------------------------------------
// API pública (compartilhada com o motor)
// ---------------------------------------------------------------------------

/// Corrige e aplica no campo (ou copia, se o campo não aceitar edição).
pub const ACAO_CORRIGIR: u32 = 1;
/// Reescreve o texto com a IA personalizada (prompt da configuração).
pub const ACAO_MELHORAR: u32 = 2;
/// Copia a seleção exatamente como está (não chama a API nem a IA).
pub const ACAO_COPIAR: u32 = 3;
/// Botão "Copiar resultado" do balão.
pub const ACAO_COPIAR_RESULTADO: u32 = 4;
/// "Apenas copiar a correção": corrige sem alterar o campo.
pub const ACAO_CORRIGIR_COPIAR: u32 = 5;
/// "Melhorar" com variantes de estilo.
pub const ACAO_MELHORAR_FORMAL: u32 = 6;
pub const ACAO_MELHORAR_SIMPLES: u32 = 7;
pub const ACAO_MELHORAR_CURTO: u32 = 8;
/// Botão "⋮" (abre o flyout de opções; não é uma ação do motor).
pub const ACAO_MAIS: u32 = 9;
/// Opções do flyout "⋮".
pub const ACAO_ABRIR_APP: u32 = 10;
pub const ACAO_DICIONARIO: u32 = 11;
pub const ACAO_PAUSAR: u32 = 12;
pub const ACAO_SAIR: u32 = 13;
/// Botão "✕": apenas fecha a barra.
pub const ACAO_FECHAR: u32 = 14;
/// "Revisar avisos…": abre a janela com a lista de avisos.
pub const ACAO_REVISAR: u32 = 15;
/// Ids `ACAO_SUGESTAO_BASE + i` aplicam **só** a alternativa `i` do primeiro
/// aviso da análise (é o que a seta do "Corrigir" oferece).
pub const ACAO_SUGESTAO_BASE: u32 = 100;
pub const MAX_SUGESTOES: usize = 32;
/// Item sem ação (avisos de "analisando…", rótulos).
pub const ACAO_NENHUMA: u32 = 0;

/// Ícones desenhados com GDI (nenhuma dependência de fonte de ícones).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icone {
    /// Nada é desenhado à esquerda do rótulo (itens dos flyouts).
    Nenhum,
    /// Marca do app ("A"), pintada pela barra — não é um botão.
    Marca,
    Corrigir,
    Estrela,
    Copiar,
    Mais,
    Fechar,
}

/// Um item da barra de ações.
#[derive(Debug, Clone)]
pub struct ItemBarra {
    pub id: u32,
    pub rotulo: String,
    pub icone: Icone,
    /// Desenha só o ícone (o rótulo continua existindo para acessibilidade).
    pub somente_icone: bool,
    /// Desenha um "▾" e abre o submenu só quando o clique é na seta.
    pub seta: bool,
    /// Traço vertical separando este item do anterior.
    pub divisor_antes: bool,
    /// Itens do flyout (vazio = sem submenu).
    pub submenu: Vec<(u32, String)>,
}

impl ItemBarra {
    /// Marca do app (não clicável).
    pub fn marca() -> Self {
        Self {
            id: 0,
            rotulo: String::new(),
            icone: Icone::Marca,
            somente_icone: true,
            seta: false,
            divisor_antes: false,
            submenu: Vec::new(),
        }
    }

    pub fn acao(id: u32, rotulo: impl Into<String>, icone: Icone) -> Self {
        Self {
            id,
            rotulo: rotulo.into(),
            icone,
            somente_icone: false,
            seta: false,
            divisor_antes: false,
            submenu: Vec::new(),
        }
    }

    pub fn com_divisor(mut self) -> Self {
        self.divisor_antes = true;
        self
    }

    pub fn somente_icone(mut self) -> Self {
        self.somente_icone = true;
        self
    }

    pub fn com_seta(mut self) -> Self {
        self.seta = true;
        self
    }

    pub fn com_submenu(mut self, itens: Vec<(u32, String)>) -> Self {
        self.submenu = itens;
        self
    }
}

#[derive(Debug, Clone)]
pub enum PopupComando {
    /// Mostra a barra de ações em (x, y) — coordenadas de tela do cursor.
    Barra { x: i32, y: i32, itens: Vec<ItemBarra> },
    /// Mostra o balão de resultado em (x, y).
    Balao { x: i32, y: i32, texto: String },
    /// Esconde tudo.
    Fechar,
}

#[derive(Debug, Clone)]
pub enum PopupEvento {
    Acao(u32),
    Fechado,
}

#[cfg(windows)]
pub use implementacao::Popups;

#[cfg(not(windows))]
pub use stub::Popups;

#[cfg(not(windows))]
mod stub {
    use super::{ItemBarra, PopupComando};
    use crate::motor::Mensagem;
    use std::sync::mpsc::Sender;

    /// Sem popups nativos: os comandos são descartados (o motor cai no caminho
    /// "corrigir direto" e o resultado aparece na janela).
    pub struct Popups {
        _canal: Sender<Mensagem>,
    }

    impl Popups {
        pub fn novo(canal: Sender<Mensagem>) -> Self {
            Self { _canal: canal }
        }
        pub fn suportado() -> bool {
            false
        }
        pub fn barra_aberta(&self) -> bool {
            false
        }
        pub fn barra(&self, _x: i32, _y: i32, _itens: Vec<ItemBarra>) {
            // mantém o tipo em uso também fora do Windows
            let _ = PopupComando::Fechar;
        }
        pub fn balao(&self, _x: i32, _y: i32, _texto: String) {}
        pub fn fechar(&self) {}
    }
}

// ---------------------------------------------------------------------------
// Windows
// ---------------------------------------------------------------------------

#[cfg(windows)]
mod implementacao {
    use super::{Icone, ItemBarra, PopupComando, PopupEvento};
    use crate::motor::Mensagem;
    use std::cell::RefCell;
    use std::sync::mpsc::{channel, Receiver, Sender};
    use std::time::Duration;
    use windows::core::{w, PCWSTR};
    use windows::Win32::Foundation::{
        COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM,
    };
    use windows::Win32::Graphics::Gdi::{
        BeginPaint, CreateFontW, CreatePen, CreateRoundRectRgn, CreateSolidBrush, DrawTextW,
        Ellipse, EndPaint, FillRect, GetDC, GetDeviceCaps, GetMonitorInfoW,
        GetStockObject, GetTextExtentPoint32W, InvalidateRect, LineTo, MonitorFromPoint, MoveToEx,
        Polygon, Polyline, ReleaseDC, RoundRect, SelectObject, SetBkColor, SetBkMode, SetTextColor,
        SetWindowRgn, CLIP_DEFAULT_PRECIS, CLEARTYPE_QUALITY, DEFAULT_CHARSET, DEFAULT_PITCH,
        DRAW_TEXT_FORMAT, DT_CENTER, DT_LEFT, DT_SINGLELINE, DT_VCENTER, FF_DONTCARE, HBRUSH, HDC,
        HFONT, HGDIOBJ, HPEN, HOLLOW_BRUSH, LOGPIXELSX, MONITORINFO, MONITOR_DEFAULTTONEAREST,
        NULL_PEN, OUT_DEFAULT_PRECIS, PAINTSTRUCT, PS_SOLID, TRANSPARENT,
    };
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_ESCAPE, VK_LBUTTON};
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetClientRect,
        GetCursorPos, GetWindowLongPtrW, GetWindowRect, GetWindowTextLengthW, GetWindowTextW,
        PeekMessageW, RegisterClassExW, SetLayeredWindowAttributes, SetTimer, SetWindowLongPtrW,
        SetWindowPos, ShowWindow, TranslateMessage, WindowFromPoint, GWLP_USERDATA, HMENU,
        LWA_ALPHA, MSG, PM_REMOVE, SWP_NOACTIVATE, SWP_SHOWWINDOW,
        SW_HIDE, SW_SHOWNOACTIVATE, WINDOW_EX_STYLE, WINDOW_STYLE, WM_CTLCOLOREDIT,
        WM_CTLCOLORSTATIC, WNDCLASSEXW, WNDCLASS_STYLES,
    };

    // Estilos/mensagens do Win32 (valores estáveis, evita depender do nome que
    // cada versão da crate `windows` dá a eles).
    const WS_POPUP: u32 = 0x8000_0000;
    const WS_CHILD: u32 = 0x4000_0000;
    const WS_VISIBLE: u32 = 0x1000_0000;
    const WS_VSCROLL: u32 = 0x0020_0000;
    const ES_MULTILINE: u32 = 0x0004;
    const ES_AUTOVSCROLL: u32 = 0x0040;
    const ES_READONLY: u32 = 0x0800;
    const WS_EX_TOPMOST: u32 = 0x0000_0008;
    const WS_EX_TOOLWINDOW: u32 = 0x0000_0080;
    const WS_EX_LAYERED: u32 = 0x0008_0000;
    const WS_EX_NOACTIVATE: u32 = 0x0800_0000;
    const WM_PAINT: u32 = 0x000F;
    const WM_ERASEBKGND: u32 = 0x0014;
    const WM_LBUTTONDOWN: u32 = 0x0201;
    const WM_TIMER: u32 = 0x0113;
    const WM_NCDESTROY: u32 = 0x0082;
    const WM_SETFONT: u32 = 0x0030;
    const WM_GETTEXT: u32 = 0x000D;
    const WM_SETTEXT: u32 = 0x000C;
    const TIMER_AUTOFECHAR: usize = 1;
    const MS_AUTO_FECHAR: u32 = 12_000;
    const MS_AUTO_FECHAR_BALAO: u32 = 20_000;
    const HWND_TOPMOST: HWND = HWND(-1isize as *mut core::ffi::c_void);
    const HWND_NULO: HWND = HWND(std::ptr::null_mut());
    const HMENU_NULO: HMENU = HMENU(std::ptr::null_mut());
    /// Marca nos dados do item (para não confundir com outras janelas filhas).
    const MAGIA: u32 = 0x4F52_5448;

    /// Paleta translúcida no estilo Apple Liquid Glass (RGB em ordem COLORREF: 0x00BBGGRR).
    const FUNDO: COLORREF = COLORREF(0x0020_1E1E); // RGB(30,30,32) - vidro fumê escuro
    const FUNDO_HOVER: COLORREF = COLORREF(0x003B_3938); // RGB(56,57,59) - realce vítreo suave
    const BORDA: COLORREF = COLORREF(0x0068_5C54); // RGB(84,92,104) - anel de brilho sutil da gota
    const DIVISOR: COLORREF = COLORREF(0x004A_4440); // RGB(64,68,74) - divisor translúcido discreto
    const TEXTO: COLORREF = COLORREF(0x00F4_F2F0); // RGB(240,242,244) - tipografia límpida
    const ICONE: COLORREF = COLORREF(0x00E6_E4E2); // RGB(226,228,230)
    const ACENTO: COLORREF = COLORREF(0x00FF_A658); // RGB(88,166,255) - Apple System Azure
    const TEXTO_BALAO: COLORREF = COLORREF(0x00F0_EEEC);

    /// Escala a partir das medidas do "design" (96 dpi).
    fn px(valor: f32, escala: f32) -> i32 {
        ((valor * escala).round() as i32).max(1)
    }

    /// Objetos GDI criados uma vez por thread do popup.
    #[derive(Clone, Copy)]
    struct Recursos {
        escala: f32,
        raio: i32,
        fonte_item: HFONT,
        fonte_marca: HFONT,
        fonte_edit: HFONT,
        pincel_fundo: HBRUSH,
        pincel_hover: HBRUSH,
        pincel_acento: HBRUSH,
        pincel_divisor: HBRUSH,
        caneta_icone: HPEN,
        caneta_acento: HPEN,
        caneta_borda: HPEN,
    }

    /// Dados de cada janela de item (guardados em GWLP_USERDATA).
    struct DadosItem {
        magia: u32,
        id: u32,
        icone: Icone,
        somente_icone: bool,
        seta: bool,
        vertical: bool,
        submenu: Vec<(u32, String)>,
    }

    struct Estado {
        canal: Sender<Mensagem>,
        recursos: Recursos,
        barra: Option<HWND>,
        flyout: Option<HWND>,
        balao: Option<HWND>,
        itens_barra: Vec<HWND>,
        itens_flyout: Vec<HWND>,
        itens_balao: Vec<HWND>,
        /// Retângulo da marca e traços divisores (pintados pela barra).
        marca: Option<RECT>,
        divisores: Vec<RECT>,
        quente: Option<HWND>,
        aberta: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    }

    thread_local! {
        static ESTADO: RefCell<Option<Estado>> = const { RefCell::new(None) };
    }

    pub struct Popups {
        tx: Sender<PopupComando>,
        /// A barra está aberta agora? (o motor usa para atualizar o submenu
        /// quando a análise fica pronta)
        aberta: std::sync::Arc<std::sync::atomic::AtomicBool>,
    }

    impl Popups {
        pub fn novo(canal: Sender<Mensagem>) -> Self {
            let (tx, rx) = channel::<PopupComando>();
            let aberta = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let aberta_thread = std::sync::Arc::clone(&aberta);
            std::thread::spawn(move || unsafe { laco_popups(canal, rx, aberta_thread) });
            Self { tx, aberta }
        }

        pub fn suportado() -> bool {
            true
        }

        /// A barra de ações está visível?
        pub fn barra_aberta(&self) -> bool {
            self.aberta.load(std::sync::atomic::Ordering::Relaxed)
        }

        pub fn barra(&self, x: i32, y: i32, itens: Vec<ItemBarra>) {
            let _ = self.tx.send(PopupComando::Barra { x, y, itens });
        }

        pub fn balao(&self, x: i32, y: i32, texto: String) {
            let _ = self.tx.send(PopupComando::Balao { x, y, texto });
        }

        pub fn fechar(&self) {
            let _ = self.tx.send(PopupComando::Fechar);
        }
    }

    fn com_estado<R>(acao: impl FnOnce(&mut Estado) -> R) -> Option<R> {
        ESTADO.with(|e| e.borrow_mut().as_mut().map(acao))
    }

    fn recursos() -> Option<Recursos> {
        com_estado(|e| e.recursos)
    }

    fn enviar(evento: PopupEvento) {
        com_estado(|e| {
            let _ = e.canal.send(Mensagem::Popup(evento));
        });
    }

    fn largo(texto: &str) -> Vec<u16> {
        texto.encode_utf16().chain(std::iter::once(0)).collect()
    }

    unsafe fn texto_janela(janela: HWND) -> String {
        let n = GetWindowTextLengthW(janela);
        if n <= 0 {
            return String::new();
        }
        let mut buffer = vec![0u16; (n + 1) as usize];
        let copiado = GetWindowTextW(janela, &mut buffer);
        String::from_utf16_lossy(&buffer[..copiado.max(0) as usize])
    }

    /// Cuidado: `GWLP_USERDATA` de janelas de **outros processos** aponta para
    /// memória alheia — nunca dereferencie sem confirmar que a janela é nossa.
    unsafe fn dados_de(janela: HWND) -> Option<&'static DadosItem> {
        if !janela_e_nossa(janela) {
            return None;
        }
        let ponteiro = GetWindowLongPtrW(janela, GWLP_USERDATA) as *const DadosItem;
        if ponteiro.is_null() {
            return None;
        }
        let dados = &*ponteiro;
        if dados.magia == MAGIA {
            Some(dados)
        } else {
            None
        }
    }

    unsafe fn janela_e_nossa(janela: HWND) -> bool {
        if janela.is_invalid() {
            return false;
        }
        let mut pid = 0u32;
        windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId(janela, Some(&mut pid));
        pid == std::process::id()
    }

    /// É um dos botões que nós criamos? (usado no realce ao passar o mouse)
    unsafe fn eh_item_nosso(janela: HWND) -> bool {
        com_estado(|e| {
            e.itens_barra.contains(&janela)
                || e.itens_flyout.contains(&janela)
                || e.itens_balao.contains(&janela)
        })
        .unwrap_or(false)
    }

    unsafe fn esconder_tudo() {
        com_estado(|e| {
            for janela in [e.barra, e.flyout, e.balao].into_iter().flatten() {
                let _ = ShowWindow(janela, SW_HIDE);
            }
            e.quente = None;
            if let Some(aberta) = &e.aberta {
                aberta.store(false, std::sync::atomic::Ordering::Relaxed);
            }
        });
    }

    unsafe fn destruir(itens: &mut Vec<HWND>) {
        for janela in itens.drain(..) {
            let _ = DestroyWindow(janela);
        }
    }

    /// Oculta a barra, o flyout e manda a ação para o motor.
    unsafe fn acionar(id: u32) {
        esconder_tudo();
        if id != super::ACAO_NENHUMA {
            enviar(PopupEvento::Acao(id));
        }
    }

    // -----------------------------------------------------------------------
    // Criação de janelas
    // -----------------------------------------------------------------------

    unsafe fn registrar_classe(pincel: HBRUSH, nome: &[u16], proc_: unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT) -> bool {
        let hinst = match GetModuleHandleW(PCWSTR::null()) {
            Ok(h) => h,
            Err(_) => return false,
        };
        let classe = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: WNDCLASS_STYLES(0),
            lpfnWndProc: Some(proc_),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: HINSTANCE(hinst.0),
            hIcon: Default::default(),
            hCursor: Default::default(),
            hbrBackground: pincel,
            lpszMenuName: PCWSTR::null(),
            lpszClassName: PCWSTR(nome.as_ptr()),
            hIconSm: Default::default(),
        };
        RegisterClassExW(&classe) != 0
    }

    unsafe fn criar_janela(classe: &[u16], proc_: unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT) -> Option<HWND> {
        let _ = proc_;
        let hinst = GetModuleHandleW(PCWSTR::null()).ok()?;
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE(WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOACTIVATE | WS_EX_LAYERED),
            PCWSTR(classe.as_ptr()),
            PCWSTR::null(),
            WINDOW_STYLE(WS_POPUP),
            0,
            0,
            10,
            10,
            HWND_NULO,
            HMENU_NULO,
            HINSTANCE(hinst.0),
            None,
        )
        .ok()?;

        // Translucidez suave estilo vidro / gota d'água Apple (234/255 ~ 92% de opacidade)
        let _ = SetLayeredWindowAttributes(
            hwnd,
            COLORREF(0),
            234,
            LWA_ALPHA,
        );
        Some(hwnd)
    }

    unsafe fn criar_edit(pai: HWND, texto: &str, x: i32, y: i32, largura: i32, altura: i32, fonte: HFONT) -> Option<HWND> {
        let t = largo(texto);
        let hinst = GetModuleHandleW(PCWSTR::null()).ok()?;
        let janela = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("EDIT"),
            PCWSTR(t.as_ptr()),
            WINDOW_STYLE(WS_CHILD | WS_VISIBLE | WS_VSCROLL | ES_MULTILINE | ES_AUTOVSCROLL | ES_READONLY),
            x,
            y,
            largura,
            altura,
            pai,
            HMENU_NULO,
            HINSTANCE(hinst.0),
            None,
        )
        .ok()?;
        let _ = windows::Win32::UI::WindowsAndMessaging::SendMessageW(
            janela,
            WM_SETFONT,
            WPARAM(fonte.0 as usize),
            LPARAM(1),
        );
        Some(janela)
    }

    /// Cria uma janela de item (barra, flyout ou botão do balão).
    #[allow(clippy::too_many_arguments)]
    unsafe fn criar_item(
        pai: HWND,
        id: u32,
        rotulo: &str,
        icone: Icone,
        somente_icone: bool,
        seta: bool,
        vertical: bool,
        submenu: Vec<(u32, String)>,
        x: i32,
        y: i32,
        largura: i32,
        altura: i32,
    ) -> Option<HWND> {
        let t = largo(rotulo);
        let hinst = GetModuleHandleW(PCWSTR::null()).ok()?;
        let janela = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("CorretorItemBarra"),
            PCWSTR(t.as_ptr()),
            WINDOW_STYLE(WS_CHILD | WS_VISIBLE),
            x,
            y,
            largura,
            altura,
            pai,
            HMENU(id as *mut core::ffi::c_void),
            HINSTANCE(hinst.0),
            None,
        )
        .ok()?;
        let dados = Box::into_raw(Box::new(DadosItem {
            magia: MAGIA,
            id,
            icone,
            somente_icone,
            seta,
            vertical,
            submenu,
        }));
        SetWindowLongPtrW(janela, GWLP_USERDATA, dados as isize);
        Some(janela)
    }

    /// Área útil do monitor onde está o cursor (evita abrir fora da tela).
    unsafe fn area_util(x: i32, y: i32) -> (i32, i32, i32, i32) {
        let monitor = MonitorFromPoint(POINT { x, y }, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            rcMonitor: RECT::default(),
            rcWork: RECT::default(),
            dwFlags: 0,
        };
        if GetMonitorInfoW(monitor, &mut info).as_bool() {
            let r = info.rcWork;
            (r.left, r.top, r.right, r.bottom)
        } else {
            (0, 0, 1280, 720)
        }
    }

    unsafe fn ajustar_na_tela(x: i32, y: i32, largura: i32, altura: i32) -> (i32, i32) {
        let (esq, topo, dir, baixo) = area_util(x, y);
        let mut px_ = x;
        let mut py_ = y;
        if px_ + largura > dir {
            px_ = (dir - largura).max(esq);
        }
        if py_ + altura > baixo {
            py_ = (y - altura - px(8.0, 1.0) * 4).max(topo);
        }
        (px_, py_)
    }

    unsafe fn mostrar(janela: HWND, x: i32, y: i32, largura: i32, altura: i32, auto_fechar_ms: u32) {
        let (px_, py_) = ajustar_na_tela(x, y, largura, altura);
        let _ = SetWindowPos(
            janela,
            HWND_TOPMOST,
            px_,
            py_,
            largura,
            altura,
            SWP_NOACTIVATE | SWP_SHOWWINDOW,
        );
        let _ = ShowWindow(janela, SW_SHOWNOACTIVATE);
        SetTimer(janela, TIMER_AUTOFECHAR, auto_fechar_ms, None);
    }

    unsafe fn aplicar_regiao(janela: HWND, largura: i32, altura: i32, raio: i32) {
        let regiao = CreateRoundRectRgn(0, 0, largura + 1, altura + 1, raio * 2, raio * 2);
        if !regiao.is_invalid() {
            let _ = SetWindowRgn(janela, regiao, true);
        }
    }

    unsafe fn cursor_dentro(janela: HWND) -> bool {
        let mut r = RECT::default();
        if GetWindowRect(janela, &mut r).is_err() {
            return false;
        }
        let mut p = POINT::default();
        if GetCursorPos(&mut p).is_err() {
            return false;
        }
        p.x >= r.left && p.x <= r.right && p.y >= r.top && p.y <= r.bottom
    }

    // -----------------------------------------------------------------------
    // Desenho (GDI)
    // -----------------------------------------------------------------------

    unsafe fn selecionar(hdc: HDC, objeto: HGDIOBJ) -> HGDIOBJ {
        SelectObject(hdc, objeto)
    }

    unsafe fn preencher_arredondado(hdc: HDC, w: i32, h: i32, raio: i32, pincel: HBRUSH) {
        let pincel_antigo = selecionar(hdc, HGDIOBJ(pincel.0));
        let caneta_antiga = selecionar(hdc, GetStockObject(NULL_PEN));
        let _ = RoundRect(hdc, 0, 0, w, h, raio * 2, raio * 2);
        selecionar(hdc, caneta_antiga);
        selecionar(hdc, pincel_antigo);
    }

    unsafe fn escrever(hdc: HDC, texto: &str, mut area: RECT, fonte: HFONT, cor: COLORREF, formato: DRAW_TEXT_FORMAT) {
        if texto.is_empty() {
            return;
        }
        let mut utf16: Vec<u16> = texto.encode_utf16().collect();
        let antiga = selecionar(hdc, HGDIOBJ(fonte.0));
        SetBkMode(hdc, TRANSPARENT);
        SetTextColor(hdc, cor);
        DrawTextW(hdc, &mut utf16, &mut area, formato);
        selecionar(hdc, antiga);
    }

    unsafe fn largura_texto(hdc: HDC, fonte: HFONT, texto: &str) -> i32 {
        let mut utf16: Vec<u16> = texto.encode_utf16().collect();
        let antiga = selecionar(hdc, HGDIOBJ(fonte.0));
        let mut tamanho = SIZE::default();
        let _ = GetTextExtentPoint32W(hdc, &mut utf16, &mut tamanho);
        selecionar(hdc, antiga);
        tamanho.cx
    }

    unsafe fn desenhar_chevron(hdc: HDC, x: i32, y: i32, largura: i32, r: &Recursos) {
        let caneta = selecionar(hdc, HGDIOBJ(r.caneta_icone.0));
        let altura = ((largura as f32) * 0.5).round().max(2.0) as i32;
        let pontos = [
            POINT { x, y: y - altura / 2 },
            POINT { x: x + largura / 2, y: y + altura / 2 },
            POINT { x: x + largura, y: y - altura / 2 },
        ];
        let _ = Polyline(hdc, &pontos);
        selecionar(hdc, caneta);
    }

    unsafe fn desenhar_icone(hdc: HDC, icone: Icone, x: i32, y: i32, lado: i32, r: &Recursos) {
        let caneta = selecionar(hdc, HGDIOBJ(r.caneta_icone.0));
        let pincel = selecionar(hdc, HGDIOBJ(r.pincel_acento.0));
        match icone {
            Icone::Estrela => {
                let cx = x + lado / 2;
                let cy = y + lado / 2;
                let raio = ((lado as f32) * 0.42).round().max(2.0) as i32;
                let cintura = ((raio as f32) * 0.30).round().max(1.0) as i32;
                let estrela = [
                    POINT { x: cx, y: cy - raio },
                    POINT { x: cx + cintura, y: cy - cintura },
                    POINT { x: cx + raio, y: cy },
                    POINT { x: cx + cintura, y: cy + cintura },
                    POINT { x: cx, y: cy + raio },
                    POINT { x: cx - cintura, y: cy + cintura },
                    POINT { x: cx - raio, y: cy },
                    POINT { x: cx - cintura, y: cy - cintura },
                ];
                selecionar(hdc, GetStockObject(NULL_PEN));
                let _ = Polygon(hdc, &estrela);
                selecionar(hdc, HGDIOBJ(r.caneta_icone.0));
                // brilho menor no canto (ar de "varinha mágica")
                let raio2 = (raio as f32 * 0.40).round().max(1.0) as i32;
                let cx2 = x + lado - raio2 - 1;
                let cy2 = y + lado - raio2 - 1;
                let c2 = (raio2 as f32 * 0.35).round().max(1.0) as i32;
                let brilho = [
                    POINT { x: cx2, y: cy2 - raio2 },
                    POINT { x: cx2 + c2, y: cy2 - c2 },
                    POINT { x: cx2 + raio2, y: cy2 },
                    POINT { x: cx2 + c2, y: cy2 + c2 },
                    POINT { x: cx2, y: cy2 + raio2 },
                    POINT { x: cx2 - c2, y: cy2 + c2 },
                    POINT { x: cx2 - raio2, y: cy2 },
                    POINT { x: cx2 - c2, y: cy2 - c2 },
                ];
                let _ = Polygon(hdc, &brilho);
            }
            Icone::Copiar => {
                let vazio = selecionar(hdc, GetStockObject(HOLLOW_BRUSH));
                let a = ((lado as f32) * 0.62).round().max(6.0) as i32;
                let desloc = ((lado as f32) * 0.10).round().max(1.0) as i32;
                let canto = ((lado as f32) * 0.16).round().max(2.0) as i32;
                let _ = RoundRect(hdc, x, y + desloc, x + a, y + a, canto * 2, canto * 2);
                let _ = RoundRect(
                    hdc,
                    x + (lado - a),
                    y + (lado - a),
                    x + lado,
                    y + lado - desloc,
                    canto * 2,
                    canto * 2,
                );
                selecionar(hdc, vazio);
            }
            Icone::Mais => {
                selecionar(hdc, GetStockObject(NULL_PEN));
                let raio = ((lado as f32) * 0.10).round().max(1.0) as i32;
                let cx = x + lado / 2;
                for fator in [0.25f32, 0.5, 0.75] {
                    let cy = y + ((lado as f32) * fator).round() as i32;
                    let _ = Ellipse(hdc, cx - raio, cy - raio, cx + raio + 1, cy + raio + 1);
                }
                selecionar(hdc, HGDIOBJ(r.caneta_icone.0));
            }
            Icone::Fechar => {
                let desloc = ((lado as f32) * 0.22).round().max(2.0) as i32;
                let _ = MoveToEx(hdc, x + desloc, y + desloc, None);
                let _ = LineTo(hdc, x + lado - desloc, y + lado - desloc);
                let _ = MoveToEx(hdc, x + lado - desloc, y + desloc, None);
                let _ = LineTo(hdc, x + desloc, y + lado - desloc);
            }
            Icone::Corrigir => {
                // "A" com um lápis (reescrever)
                let area = RECT {
                    left: x - px(1.0, r.escala),
                    top: y - px(1.0, r.escala),
                    right: x + lado,
                    bottom: y + lado + px(1.0, r.escala),
                };
                escrever(hdc, "A", area, r.fonte_marca, ICONE, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
                let caneta_acento = selecionar(hdc, HGDIOBJ(r.caneta_acento.0));
                let x0 = x + lado - px(5.0, r.escala);
                let y0 = y + lado - px(2.0, r.escala);
                let _ = MoveToEx(hdc, x0, y0, None);
                let _ = LineTo(hdc, x + lado - px(1.0, r.escala), y + lado - px(6.0, r.escala));
                selecionar(hdc, caneta_acento);
            }
            Icone::Marca | Icone::Nenhum => {}
        }
        selecionar(hdc, pincel);
        selecionar(hdc, caneta);
    }

    unsafe fn desenhar_marca(hdc: HDC, rc: RECT, r: &Recursos) {
        let mut area = rc;
        area.bottom -= px(3.0, r.escala);
        escrever(hdc, "A", area, r.fonte_marca, ACENTO, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
        // ponto de destaque (como no logotipo)
        let pincel = selecionar(hdc, HGDIOBJ(r.pincel_acento.0));
        let caneta = selecionar(hdc, GetStockObject(NULL_PEN));
        let raio = px(1.6, r.escala);
        let cx = rc.left + px(6.0, r.escala);
        let cy = rc.bottom - px(4.0, r.escala);
        let _ = Ellipse(hdc, cx - raio, cy - raio, cx + raio + 1, cy + raio + 1);
        selecionar(hdc, caneta);
        selecionar(hdc, pincel);
    }

    unsafe fn esta_quente(janela: HWND) -> bool {
        com_estado(|e| e.quente == Some(janela)).unwrap_or(false)
    }

    /// Pintura de um item (barra, flyout ou botão do balão).
    unsafe fn pintar_item(janela: HWND, dados: &DadosItem) {
        let mut ps = PAINTSTRUCT::default();
        let hdc = BeginPaint(janela, &mut ps);
        if let Some(r) = recursos() {
            let mut rc = RECT::default();
            let _ = GetClientRect(janela, &mut rc);
            let w = rc.right - rc.left;
            let h = rc.bottom - rc.top;
            let raio_hover = if dados.vertical {
                px(8.0, r.escala)
            } else {
                h / 2
            };
            if esta_quente(janela) {
                preencher_arredondado(hdc, w, h, raio_hover, r.pincel_hover);
            } else {
                FillRect(hdc, &rc, r.pincel_fundo);
            }

            let pad = px(10.0, r.escala);
            let lado = px(16.0, r.escala);
            let espaco = px(7.0, r.escala);
            let seta_largura = px(9.0, r.escala);
            let mut x = pad;
            let mut direita = w - pad;

            if dados.vertical {
                // flyout: rótulo à esquerda, sem ícone
                if dados.icone != Icone::Nenhum {
                    desenhar_icone(hdc, dados.icone, x, (h - lado) / 2, lado, &r);
                    x += lado + espaco;
                }
                let area = RECT { left: x, top: 0, right: direita, bottom: h };
                escrever(hdc, &texto_janela(janela), area, r.fonte_item, TEXTO, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
            } else {
                if !dados.submenu.is_empty() && dados.seta {
                    direita -= seta_largura + espaco;
                }
                if dados.icone != Icone::Nenhum {
                    desenhar_icone(hdc, dados.icone, x, (h - lado) / 2, lado, &r);
                    if !dados.somente_icone {
                        x += lado + espaco;
                    }
                }
                if !dados.somente_icone {
                    let area = RECT { left: x, top: 0, right: direita, bottom: h };
                    escrever(hdc, &texto_janela(janela), area, r.fonte_item, TEXTO, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
                }
                if !dados.submenu.is_empty() && dados.seta {
                    desenhar_chevron(hdc, w - pad - seta_largura, h / 2, seta_largura, &r);
                }
            }
        }
        let _ = EndPaint(janela, &ps);
    }

    /// Pintura do fundo (barra, flyout e balão) + marca e divisores.
    unsafe fn pintar_fundo(janela: HWND) {
        let mut ps = PAINTSTRUCT::default();
        let hdc = BeginPaint(janela, &mut ps);
        if let Some(r) = recursos() {
            let mut rc = RECT::default();
            let _ = GetClientRect(janela, &mut rc);
            let w = rc.right;
            let h = rc.bottom;
            FillRect(hdc, &rc, r.pincel_fundo);
            // Borda cristalina de gota d'água / vidro suave
            let raio = if w > h * 2 { h / 2 } else { r.raio };
            let pincel_antigo = selecionar(hdc, GetStockObject(HOLLOW_BRUSH));
            let caneta_antiga = selecionar(hdc, HGDIOBJ(r.caneta_borda.0));
            let _ = RoundRect(hdc, 0, 0, w, h, raio * 2, raio * 2);
            selecionar(hdc, caneta_antiga);
            selecionar(hdc, pincel_antigo);

            let (marca, divisores) = com_estado(|e| (e.marca, e.divisores.clone())).unwrap_or((None, Vec::new()));
            for divisor in divisores {
                FillRect(hdc, &divisor, r.pincel_divisor);
            }
            if let Some(retangulo) = marca {
                desenhar_marca(hdc, retangulo, &r);
            }
        }
        let _ = EndPaint(janela, &ps);
    }

    // -----------------------------------------------------------------------
    // Procedimentos de janela
    // -----------------------------------------------------------------------

    unsafe fn abrir_flyout(origem: HWND, submenu: &[(u32, String)]) {
        if submenu.is_empty() {
            return;
        }
        let flyout = match com_estado(|e| e.flyout).flatten() {
            Some(j) => j,
            None => return,
        };
        let r = match recursos() {
            Some(r) => r,
            None => return,
        };

        let mut antigos = Vec::new();
        com_estado(|e| antigos = std::mem::take(&mut e.itens_flyout));
        destruir(&mut antigos);

        let margem = px(4.0, r.escala);
        let pad = px(12.0, r.escala);
        let altura_item = px(28.0, r.escala);
        let hdc = GetDC(flyout);
        let mut largura = px(180.0, r.escala);
        for (_, rotulo) in submenu {
            let l = largura_texto(hdc, r.fonte_item, rotulo) + pad * 2;
            if l > largura {
                largura = l;
            }
        }
        let _ = ReleaseDC(flyout, hdc);
        let altura = submenu.len() as i32 * altura_item + margem * 2;

        let mut novos = Vec::new();
        for (indice, (id, rotulo)) in submenu.iter().enumerate() {
            if let Some(janela) = criar_item(
                flyout,
                *id,
                rotulo,
                Icone::Nenhum,
                false,
                false,
                true,
                Vec::new(),
                margem,
                margem + indice as i32 * altura_item,
                largura - margem * 2,
                altura_item,
            ) {
                novos.push(janela);
            }
        }
        com_estado(|e| e.itens_flyout = novos);

        let mut retangulo = RECT::default();
        let _ = GetWindowRect(origem, &mut retangulo);
        aplicar_regiao(flyout, largura, altura, r.raio);
        mostrar(
            flyout,
            retangulo.left,
            retangulo.bottom + px(4.0, r.escala),
            largura,
            altura,
            MS_AUTO_FECHAR,
        );
    }

    unsafe extern "system" fn wndproc_item(janela: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        match msg {
            WM_PAINT => {
                if let Some(dados) = dados_de(janela) {
                    pintar_item(janela, dados);
                } else {
                    let mut ps = PAINTSTRUCT::default();
                    let _ = BeginPaint(janela, &mut ps);
                    let _ = EndPaint(janela, &ps);
                }
                LRESULT(0)
            }
            WM_ERASEBKGND => LRESULT(1),
            WM_LBUTTONDOWN => {
                if let Some(dados) = dados_de(janela) {
                    let id = dados.id;
                    let tem_submenu = !dados.submenu.is_empty();
                    let seta = dados.seta;
                    let submenu = dados.submenu.clone();
                    let mut rc = RECT::default();
                    let _ = GetClientRect(janela, &mut rc);
                    let x = (lparam.0 & 0xFFFF) as u16 as i16 as i32;
                    let zona = px(24.0, recursos().map(|r| r.escala).unwrap_or(1.0));
                    let no_canto = x >= rc.right - zona;
                    if tem_submenu && (!seta || no_canto) {
                        abrir_flyout(janela, &submenu);
                    } else {
                        acionar(id);
                    }
                }
                LRESULT(0)
            }
            WM_NCDESTROY => {
                let ponteiro = GetWindowLongPtrW(janela, GWLP_USERDATA) as *mut DadosItem;
                if !ponteiro.is_null() {
                    drop(Box::from_raw(ponteiro));
                }
                LRESULT(0)
            }
            WM_GETTEXT | WM_SETTEXT => DefWindowProcW(janela, msg, wparam, lparam),
            _ => DefWindowProcW(janela, msg, wparam, lparam),
        }
    }

    unsafe extern "system" fn wndproc_fundo(janela: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        match msg {
            WM_PAINT => {
                pintar_fundo(janela);
                LRESULT(0)
            }
            WM_ERASEBKGND => LRESULT(1),
            WM_TIMER => {
                esconder_tudo();
                LRESULT(0)
            }
            WM_CTLCOLOREDIT | WM_CTLCOLORSTATIC => {
                // deixa o campo do balão no mesmo tema escuro
                if let Some(r) = recursos() {
                    let hdc = HDC(wparam.0 as *mut core::ffi::c_void);
                    SetTextColor(hdc, TEXTO_BALAO);
                    SetBkColor(hdc, FUNDO);
                    return LRESULT(r.pincel_fundo.0 as isize);
                }
                DefWindowProcW(janela, msg, wparam, lparam)
            }
            _ => DefWindowProcW(janela, msg, wparam, lparam),
        }
    }

    // -----------------------------------------------------------------------
    // Montagem da barra e do balão
    // -----------------------------------------------------------------------

    unsafe fn montar_barra(janela: HWND, x: i32, y: i32, itens: Vec<ItemBarra>) {
        let r = match recursos() {
            Some(r) => r,
            None => return,
        };
        let mut antigos = Vec::new();
        com_estado(|e| antigos = std::mem::take(&mut e.itens_barra));
        destruir(&mut antigos);

        let margem = px(4.0, r.escala);
        let pad = px(11.0, r.escala);
        let lado = px(16.0, r.escala);
        let espaco = px(7.0, r.escala);
        let seta_largura = px(10.0, r.escala);
        let separador = px(9.0, r.escala);
        let largura_marca = px(34.0, r.escala);
        let altura_item = px(32.0, r.escala);
        let hdc = GetDC(janela);

        let mut larguras = Vec::with_capacity(itens.len());
        for item in &itens {
            if item.icone == Icone::Marca && item.id == 0 {
                larguras.push(largura_marca);
                continue;
            }
            let texto_l = if item.somente_icone || item.rotulo.is_empty() {
                0
            } else {
                largura_texto(hdc, r.fonte_item, &item.rotulo)
            };
            let mut largura = pad * 2;
            if item.icone != Icone::Nenhum {
                largura += lado;
                if texto_l > 0 {
                    largura += espaco;
                }
            }
            largura += texto_l;
            if !item.submenu.is_empty() && item.seta {
                largura += espaco + seta_largura;
            }
            larguras.push(largura.max(px(28.0, r.escala)));
        }
        let _ = ReleaseDC(janela, hdc);

        let divisores = itens.iter().filter(|i| i.divisor_antes).count() as i32;
        let largura_total = larguras.iter().sum::<i32>() + margem * 2 + divisores * separador;
        let altura_total = altura_item + margem * 2;

        let mut novos = Vec::new();
        let mut marca = None;
        let mut tracos: Vec<RECT> = Vec::new();
        let mut cursor_x = margem;
        for (indice, item) in itens.iter().enumerate() {
            if item.divisor_antes {
                let centro = cursor_x + separador / 2;
                tracos.push(RECT {
                    left: centro,
                    top: altura_total / 4,
                    right: centro + 1,
                    bottom: altura_total - altura_total / 4,
                });
                cursor_x += separador;
            }
            if item.icone == Icone::Marca && item.id == 0 {
                marca = Some(RECT {
                    left: cursor_x,
                    top: margem,
                    right: cursor_x + larguras[indice],
                    bottom: altura_total - margem,
                });
                cursor_x += larguras[indice];
                continue;
            }
            if let Some(janela_item) = criar_item(
                janela,
                item.id,
                &item.rotulo,
                item.icone,
                item.somente_icone,
                item.seta,
                false,
                item.submenu.clone(),
                cursor_x,
                margem,
                larguras[indice],
                altura_item,
            ) {
                novos.push(janela_item);
            }
            cursor_x += larguras[indice];
        }

        com_estado(|e| {
            e.itens_barra = novos;
            e.marca = marca;
            e.divisores = tracos;
            if let Some(aberta) = &e.aberta {
                aberta.store(true, std::sync::atomic::Ordering::Relaxed);
            }
        });
        let raio_capsula = altura_total / 2;
        aplicar_regiao(janela, largura_total, altura_total, raio_capsula);
        mostrar(
            janela,
            x + px(8.0, r.escala),
            y + px(16.0, r.escala),
            largura_total,
            altura_total,
            MS_AUTO_FECHAR,
        );
    }

    unsafe fn montar_balao(janela: HWND, x: i32, y: i32, texto: String) {
        let r = match recursos() {
            Some(r) => r,
            None => return,
        };
        let mut antigos = Vec::new();
        com_estado(|e| antigos = std::mem::take(&mut e.itens_balao));
        destruir(&mut antigos);

        let margem = px(12.0, r.escala);
        let largura = px(520.0, r.escala);
        let altura_texto = px(132.0, r.escala);
        let altura_botao = px(32.0, r.escala);
        let altura = altura_texto + altura_botao + margem * 3;

        let mut novos = Vec::new();
        if let Some(edit) = criar_edit(janela, &texto, margem, margem, largura - margem * 2, altura_texto, r.fonte_edit)
        {
            novos.push(edit);
        }

        let hdc = GetDC(janela);
        let largura_botao = largura_texto(hdc, r.fonte_item, "Copiar resultado")
            + px(11.0, r.escala) * 2
            + px(16.0, r.escala)
            + px(7.0, r.escala);
        let _ = ReleaseDC(janela, hdc);

        if let Some(botao) = criar_item(
            janela,
            super::ACAO_COPIAR_RESULTADO,
            "Copiar resultado",
            Icone::Copiar,
            false,
            false,
            false,
            Vec::new(),
            margem,
            altura_texto + margem * 2,
            largura_botao,
            altura_botao,
        ) {
            novos.push(botao);
        }

        com_estado(|e| {
            e.itens_balao = novos;
            e.marca = None;
            e.divisores.clear();
        });
        aplicar_regiao(janela, largura, altura, px(16.0, r.escala));
        mostrar(janela, x + px(8.0, r.escala), y + px(16.0, r.escala), largura, altura, MS_AUTO_FECHAR_BALAO);
    }

    // -----------------------------------------------------------------------
    // Laço principal
    // -----------------------------------------------------------------------

    unsafe fn atualizar_quente() {
        let mut p = POINT::default();
        if GetCursorPos(&mut p).is_err() {
            return;
        }
        let sob = WindowFromPoint(p);
        let alvo = if eh_item_nosso(sob) { Some(sob) } else { None };
        let anterior = com_estado(|e| e.quente).flatten();
        if anterior != alvo {
            com_estado(|e| e.quente = alvo);
            for janela in [anterior, alvo].into_iter().flatten() {
                let _ = InvalidateRect(janela, None, false);
            }
        }
    }

    unsafe fn cursor_dentro_de_alguma() -> bool {
        let janelas = com_estado(|e| {
            [e.barra, e.flyout, e.balao]
                .into_iter()
                .flatten()
                .filter(|j| windows::Win32::UI::WindowsAndMessaging::IsWindowVisible(*j).as_bool())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
        janelas.into_iter().any(|j| cursor_dentro(j))
    }

    unsafe fn criar_recursos() -> Recursos {
        let dpi = {
            let hdc = GetDC(HWND_NULO);
            let valor = if hdc.is_invalid() { 96 } else { GetDeviceCaps(hdc, LOGPIXELSX) };
            let _ = ReleaseDC(HWND_NULO, hdc);
            valor.max(96) as f32
        };
        let escala = dpi / 96.0;
        let altura = |pontos: f32| -((pontos * dpi / 72.0).round() as i32);
        let fonte = |pontos: f32, peso: i32| unsafe {
            CreateFontW(
                altura(pontos),
                0,
                0,
                0,
                peso,
                0,
                0,
                0,
                DEFAULT_CHARSET.0 as u32,
                OUT_DEFAULT_PRECIS.0 as u32,
                CLIP_DEFAULT_PRECIS.0 as u32,
                CLEARTYPE_QUALITY.0 as u32,
                (DEFAULT_PITCH.0 as u32) | (FF_DONTCARE.0 as u32),
                w!("Segoe UI"),
            )
        };
        Recursos {
            escala,
            raio: px(14.0, escala),
            fonte_item: fonte(9.0, 600),
            fonte_marca: fonte(11.0, 700),
            fonte_edit: fonte(10.0, 400),
            pincel_fundo: CreateSolidBrush(FUNDO),
            pincel_hover: CreateSolidBrush(FUNDO_HOVER),
            pincel_acento: CreateSolidBrush(ACENTO),
            pincel_divisor: CreateSolidBrush(DIVISOR),
            caneta_icone: CreatePen(PS_SOLID, px(1.0, escala).max(1) + 1, ICONE),
            caneta_acento: CreatePen(PS_SOLID, px(1.0, escala).max(1) + 1, ACENTO),
            caneta_borda: CreatePen(PS_SOLID, 1, BORDA),
        }
    }

    unsafe fn laco_popups(
        canal: Sender<Mensagem>,
        rx: Receiver<PopupComando>,
        aberta: std::sync::Arc<std::sync::atomic::AtomicBool>,
    ) {
        let recursos = criar_recursos();
        let classe_barra = largo("CorretorMenuPopup");
        let classe_flyout = largo("CorretorFlyoutPopup");
        let classe_balao = largo("CorretorBalaoPopup");
        let classe_item = largo("CorretorItemBarra");

        if !registrar_classe(recursos.pincel_fundo, &classe_barra, wndproc_fundo) {
            return;
        }
        if !registrar_classe(recursos.pincel_fundo, &classe_flyout, wndproc_fundo) {
            return;
        }
        if !registrar_classe(recursos.pincel_fundo, &classe_balao, wndproc_fundo) {
            return;
        }
        if !registrar_classe(recursos.pincel_fundo, &classe_item, wndproc_item) {
            return;
        }

        let barra = criar_janela(&classe_barra, wndproc_fundo);
        let flyout = criar_janela(&classe_flyout, wndproc_fundo);
        let balao = criar_janela(&classe_balao, wndproc_fundo);

        ESTADO.with(|e| {
            *e.borrow_mut() = Some(Estado {
                canal,
                recursos,
                barra,
                flyout,
                balao,
                itens_barra: Vec::new(),
                itens_flyout: Vec::new(),
                itens_balao: Vec::new(),
                marca: None,
                divisores: Vec::new(),
                quente: None,
                aberta: Some(aberta),
            });
        });

        let mut botao_antes = false;
        let mut esc_antes = false;

        loop {
            let mut msg = MSG::default();
            while PeekMessageW(&mut msg, HWND_NULO, 0, 0, PM_REMOVE).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            while let Ok(comando) = rx.try_recv() {
                match comando {
                    PopupComando::Barra { x, y, itens } => {
                        if let Some(janela) = barra {
                            let _ = ShowWindow(janela, SW_HIDE);
                            montar_barra(janela, x, y, itens);
                        }
                    }
                    PopupComando::Balao { x, y, texto } => {
                        if let Some(janela) = balao {
                            let _ = ShowWindow(janela, SW_HIDE);
                            montar_balao(janela, x, y, texto);
                        }
                    }
                    PopupComando::Fechar => esconder_tudo(),
                }
            }

            atualizar_quente();

            // Como o popup nunca recebe ativação, fechamos por clique fora e ESC.
            let botao = (GetAsyncKeyState(VK_LBUTTON.0 as i32) & 0x8000u16 as i16) != 0;
            if botao && !botao_antes && !cursor_dentro_de_alguma() {
                esconder_tudo();
            }
            botao_antes = botao;

            let esc = (GetAsyncKeyState(VK_ESCAPE.0 as i32) & 0x8000u16 as i16) != 0;
            if esc && !esc_antes {
                let visivel = com_estado(|e| {
                    [e.barra, e.flyout, e.balao]
                        .into_iter()
                        .flatten()
                        .any(|j| windows::Win32::UI::WindowsAndMessaging::IsWindowVisible(j).as_bool())
                })
                .unwrap_or(false);
                if visivel {
                    esconder_tudo();
                    enviar(PopupEvento::Fechado);
                }
            }
            esc_antes = esc;

            std::thread::sleep(Duration::from_millis(20));
        }
    }
}
