#[allow(unused_imports)]
use std::path::PathBuf;

pub fn configurar(habilitar: bool) -> Result<String, String> {
    if habilitar {
        ativar()
    } else {
        desativar()
    }
}

pub fn sincronizar(habilitar: bool) {
    let _ = configurar(habilitar);
}

// ---------------------------------------------------------------------------
// Windows (Registro HKCU\Software\Microsoft\Windows\CurrentVersion\Run)
// ---------------------------------------------------------------------------
#[cfg(target_os = "windows")]
pub fn ativar() -> Result<String, String> {
    let exe = std::env::current_exe().map_err(|e| format!("Não foi possível obter o caminho do executável: {}", e))?;
    let exe_str = format!("\"{}\"", exe.to_string_lossy());

    let mut cmd = std::process::Command::new("reg");
    cmd.args(&[
        "add",
        "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run",
        "/v",
        "CorretorUniversal",
        "/t",
        "REG_SZ",
        "/d",
        &exe_str,
        "/f",
    ]);

    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    cmd.creation_flags(CREATE_NO_WINDOW);

    let status = cmd.status().map_err(|e| format!("Erro ao executar reg.exe: {}", e))?;
    if status.success() {
        Ok("Inicialização automática ativada no Windows com sucesso.".to_string())
    } else {
        Err("Falha ao registrar inicialização no Registro do Windows.".to_string())
    }
}

#[cfg(target_os = "windows")]
pub fn desativar() -> Result<String, String> {
    let mut cmd = std::process::Command::new("reg");
    cmd.args(&[
        "delete",
        "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run",
        "/v",
        "CorretorUniversal",
        "/f",
    ]);

    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    cmd.creation_flags(CREATE_NO_WINDOW);

    let _ = cmd.status();
    Ok("Inicialização automática desativada no Windows.".to_string())
}

#[allow(dead_code)]
#[cfg(target_os = "windows")]
pub fn esta_habilitado() -> bool {
    let mut cmd = std::process::Command::new("reg");
    cmd.args(&[
        "query",
        "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run",
        "/v",
        "CorretorUniversal",
    ]);

    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    cmd.creation_flags(CREATE_NO_WINDOW);

    cmd.output().map(|o| o.status.success()).unwrap_or(false)
}

// ---------------------------------------------------------------------------
// Linux (XDG Autostart Specification)
// ---------------------------------------------------------------------------
#[cfg(target_os = "linux")]
fn caminho_desktop_linux() -> Result<PathBuf, String> {
    let dir = if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
        PathBuf::from(xdg).join("autostart")
    } else if let Some(home) = std::env::var_os("HOME") {
        PathBuf::from(home).join(".config").join("autostart")
    } else {
        return Err("Variável de ambiente HOME não encontrada".to_string());
    };
    Ok(dir.join("corretor_universal.desktop"))
}

#[cfg(target_os = "linux")]
pub fn ativar() -> Result<String, String> {
    let caminho = caminho_desktop_linux()?;
    if let Some(parent) = caminho.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("Erro ao criar pasta autostart: {}", e))?;
    }
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let conteudo = format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Version=1.0\n\
         Name=Corretor Ortográfico Universal\n\
         Comment=Corretor Universal de Texto com IA\n\
         Exec=\"{}\"\n\
         Terminal=false\n\
         StartupNotify=false\n\
         X-GNOME-Autostart-enabled=true\n",
        exe.to_string_lossy()
    );
    std::fs::write(&caminho, conteudo).map_err(|e| format!("Erro ao escrever corretor_universal.desktop: {}", e))?;
    Ok("Inicialização automática ativada no Linux (~/.config/autostart).".to_string())
}

#[cfg(target_os = "linux")]
pub fn desativar() -> Result<String, String> {
    let caminho = caminho_desktop_linux()?;
    if caminho.exists() {
        let _ = std::fs::remove_file(caminho);
    }
    Ok("Inicialização automática desativada no Linux.".to_string())
}

#[cfg(target_os = "linux")]
pub fn esta_habilitado() -> bool {
    caminho_desktop_linux().map(|p| p.exists()).unwrap_or(false)
}

// ---------------------------------------------------------------------------
// macOS (LaunchAgents)
// ---------------------------------------------------------------------------
#[cfg(target_os = "macos")]
fn caminho_plist_macos() -> Result<PathBuf, String> {
    if let Some(home) = std::env::var_os("HOME") {
        Ok(PathBuf::from(home).join("Library").join("LaunchAgents").join("com.corretoruniversal.app.plist"))
    } else {
        Err("Variável de ambiente HOME não encontrada".to_string())
    }
}

#[cfg(target_os = "macos")]
pub fn ativar() -> Result<String, String> {
    let caminho = caminho_plist_macos()?;
    if let Some(parent) = caminho.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("Erro ao criar pasta LaunchAgents: {}", e))?;
    }
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let conteudo = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
         <plist version=\"1.0\">\n\
         <dict>\n\
             <key>Label</key>\n\
             <string>com.corretoruniversal.app</string>\n\
             <key>ProgramArguments</key>\n\
             <array>\n\
                 <string>{}</string>\n\
             </array>\n\
             <key>RunAtLoad</key>\n\
             <true/>\n\
             <key>ProcessType</key>\n\
             <string>Interactive</string>\n\
         </dict>\n\
         </plist>\n",
        exe.to_string_lossy()
    );
    std::fs::write(&caminho, conteudo).map_err(|e| format!("Erro ao salvar LaunchAgent plist: {}", e))?;
    let _ = std::process::Command::new("launchctl")
        .args(&["load", &caminho.to_string_lossy()])
        .status();
    Ok("Inicialização automática ativada no macOS (~/Library/LaunchAgents).".to_string())
}

#[cfg(target_os = "macos")]
pub fn desativar() -> Result<String, String> {
    let caminho = caminho_plist_macos()?;
    if caminho.exists() {
        let _ = std::process::Command::new("launchctl")
            .args(&["unload", &caminho.to_string_lossy()])
            .status();
        let _ = std::fs::remove_file(caminho);
    }
    Ok("Inicialização automática desativada no macOS.".to_string())
}

#[cfg(target_os = "macos")]
pub fn esta_habilitado() -> bool {
    caminho_plist_macos().map(|p| p.exists()).unwrap_or(false)
}

// ---------------------------------------------------------------------------
// Fallback para outros sistemas
// ---------------------------------------------------------------------------
#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
pub fn ativar() -> Result<String, String> {
    Err("Inicialização automática não suportada neste sistema operacional.".to_string())
}

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
pub fn desativar() -> Result<String, String> {
    Ok("Não suportado.".to_string())
}

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
pub fn esta_habilitado() -> bool {
    false
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn test_autostart_ciclo() {
        let res = ativar();
        assert!(res.is_ok(), "Ativação deve funcionar: {:?}", res);

        #[cfg(target_os = "windows")]
        assert!(esta_habilitado(), "Deve estar habilitado após ativar()");

        let res_des = desativar();
        assert!(res_des.is_ok(), "Desativação deve funcionar: {:?}", res_des);

        #[cfg(target_os = "windows")]
        assert!(!esta_habilitado(), "Não deve estar habilitado após desativar()");
    }
}
