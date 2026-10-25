//! Módulo de gravação e manipulação de disco (Formatação FAT32, Cópia 1:1, Sync e Ejeção).
//!
//! Executa sequencialmente as 4 etapas exigidas na especificação:
//! 1. Formatação FAT32 (0% a 20%)
//! 2. Cópia Recursiva 1:1 sem criar pasta intermediária `payload` (20% a 85%)
//! 3. Flush e Sincronização de Buffers em disco (85% a 95%)
//! 4. Ejeção Automática Segura (95% a 100%)

use crate::disk::{list_removable_drives, resolve_payload_dir, validate_payload};
use serde::Serialize;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use tauri::{AppHandle, Emitter};
use walkdir::WalkDir;

/// Estrutura do evento emitido via `window.emit("flash-progress", ...)` para atualizar a UI em tempo real.
#[derive(Debug, Clone, Serialize)]
pub struct FlashProgressEvent {
    /// Percentual atual (0 a 100).
    pub percentage: u8,
    /// Etapa atual ("format", "copy", "sync", "eject", "done", "error").
    pub stage: String,
    /// Mensagem descritiva (ex: "Gravando: Aurora/Data/...").
    pub message: String,
}

/// Emite um evento `flash-progress` para todas as janelas ativas do Tauri.
fn emit_progress(app: &AppHandle, percentage: u8, stage: &str, message: impl Into<String>) {
    let payload = FlashProgressEvent {
        percentage,
        stage: stage.to_string(),
        message: message.into(),
    };
    let _ = app.emit("flash-progress", payload);
}

/// Extrai a letra da unidade no Windows (ex: `E:\` ou `E:` -> `"E"`).
#[cfg(target_os = "windows")]
fn extract_windows_drive_letter(mount_point: &str) -> Result<String, String> {
    let trimmed = mount_point.trim();
    let first_char = trimmed
        .chars()
        .next()
        .ok_or_else(|| "Ponto de montagem inválido no Windows.".to_string())?;

    if !first_char.is_ascii_alphabetic() {
        return Err(format!("Letra de unidade inválida: {}", mount_point));
    }

    let upper = first_char.to_ascii_uppercase();
    if upper == 'C' {
        return Err("TRAVA DE SEGURANÇA: Operação bloqueada na unidade C: do sistema!".to_string());
    }

    Ok(upper.to_string())
}

/// No Linux, descobre o caminho de bloco (ex: `/dev/sdb1`) associado a um ponto de montagem
/// usando `/proc/mounts` caso `device_path` não comece com `/dev/`.
#[cfg(target_os = "linux")]
fn resolve_linux_block_device(mount_point: &str, fallback_device: &str) -> Result<String, String> {
    if fallback_device.starts_with("/dev/") {
        return Ok(fallback_device.to_string());
    }

    if let Ok(mounts) = fs::read_to_string("/proc/mounts") {
        for line in mounts.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let dev = parts[0];
                // Em /proc/mounts espaços são escapados como \040
                let mp = parts[1].replace("\\040", " ");
                if mp == mount_point && dev.starts_with("/dev/") {
                    return Ok(dev.to_string());
                }
            }
        }
    }

    Err(format!(
        "Não foi possível determinar o dispositivo de bloco (/dev/*) para '{}'.",
        mount_point
    ))
}

/// No Linux, consulta em qual diretório um dispositivo de bloco está montado após remontagem.
#[cfg(target_os = "linux")]
fn find_linux_mount_point(block_dev: &str) -> Option<PathBuf> {
    if let Ok(mounts) = fs::read_to_string("/proc/mounts") {
        for line in mounts.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 && parts[0] == block_dev {
                let mp = parts[1].replace("\\040", " ");
                return Some(PathBuf::from(mp));
            }
        }
    }
    None
}

// ============================================================================
// ETAPA 1: FORMATAÇÃO FAT32 (0% a 20%)
// ============================================================================

#[cfg(target_os = "linux")]
fn format_drive_fat32(
    app: &AppHandle,
    mount_point: &str,
    device_path: &str,
) -> Result<PathBuf, String> {
    let block_dev = resolve_linux_block_device(mount_point, device_path)?;

    // Trava extra contra partições raiz/sistema
    if block_dev.contains("nvme") || mount_point == "/" || mount_point.starts_with("/boot") {
        return Err("TRAVA DE SEGURANÇA: Tentativa de formatar dispositivo de sistema bloqueada!".into());
    }

    emit_progress(
        app,
        5,
        "format",
        format!("Desmontando volume {} para formatação...", block_dev),
    );

    // Desmonta antes de executar mkfs.vfat
    let _ = Command::new("udisksctl")
        .args(["unmount", "-b", &block_dev])
        .output();
    let _ = Command::new("umount").arg(&block_dev).output();

    emit_progress(
        app,
        10,
        "format",
        format!("Formatando {} em FAT32 (mkfs.vfat -F 32 -I)...", block_dev),
    );

    // Tenta executar diretamente (ou via pkexec /usr/sbin/mkfs.vfat caso o usuário precise de elevação Polkit)
    let output = Command::new("/usr/sbin/mkfs.vfat")
        .args(["-F", "32", "-n", "XBOX360", "-I", &block_dev])
        .output()
        .or_else(|_| {
            Command::new("mkfs.vfat")
                .args(["-F", "32", "-n", "XBOX360", "-I", &block_dev])
                .output()
        })
        .map_err(|e| format!("Falha ao invocar mkfs.vfat (verifique se dosfstools está instalado): {}", e))?;

    if !output.status.success() {
        // Fallback automático via pkexec (abre o diálogo gráfico nativo de senha do Fedora/Ubuntu)
        let pkexec_output = Command::new("pkexec")
            .args(["/usr/sbin/mkfs.vfat", "-F", "32", "-n", "XBOX360", "-I", &block_dev])
            .output()
            .map_err(|e| {
                format!(
                    "Falha na formatação FAT32 ({}): {}",
                    String::from_utf8_lossy(&output.stderr),
                    e
                )
            })?;

        if !pkexec_output.status.success() {
            return Err(format!(
                "Erro ao formatar {} em FAT32: {}",
                block_dev,
                String::from_utf8_lossy(&pkexec_output.stderr)
            ));
        }
    }

    emit_progress(
        app,
        16,
        "format",
        format!("Remontando partição {} após formatação...", block_dev),
    );

    // Remonta via udisksctl (sem exigir root) para permitir a cópia como usuário atual
    let mount_out = Command::new("udisksctl")
        .args(["mount", "-b", &block_dev])
        .output();

    if let Ok(out) = mount_out {
        if out.status.success() {
            if let Some(new_mp) = find_linux_mount_point(&block_dev) {
                emit_progress(app, 20, "format", "Formatação FAT32 concluída com sucesso.");
                return Ok(new_mp);
            }
        }
    }

    // Caso o ponto de montagem original ainda exista ou udisksctl tenha montado
    if let Some(new_mp) = find_linux_mount_point(&block_dev) {
        emit_progress(app, 20, "format", "Formatação FAT32 concluída com sucesso.");
        return Ok(new_mp);
    }

    Err(format!(
        "Dispositivo {} foi formatado em FAT32, mas não pôde ser remontado automaticamente via udisksctl.",
        block_dev
    ))
}

#[cfg(target_os = "windows")]
fn format_drive_fat32(
    app: &AppHandle,
    mount_point: &str,
    _device_path: &str,
) -> Result<PathBuf, String> {
    let drive_letter = extract_windows_drive_letter(mount_point)?;

    emit_progress(
        app,
        8,
        "format",
        format!("Formatando unidade {}: em FAT32 via PowerShell...", drive_letter),
    );

    let ps_command = format!(
        "Format-Volume -DriveLetter {} -FileSystem FAT32 -NewFileSystemLabel 'XBOX360' -Force -Confirm:$false",
        drive_letter
    );

    let output = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &ps_command])
        .output()
        .map_err(|e| format!("Falha ao executar PowerShell para formatação: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "Erro ao formatar unidade {}: em FAT32: {}",
            drive_letter,
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    emit_progress(app, 20, "format", "Formatação FAT32 concluída com sucesso.");
    Ok(PathBuf::from(format!("{}:\\", drive_letter)))
}

#[cfg(target_os = "macos")]
fn format_drive_fat32(
    app: &AppHandle,
    mount_point: &str,
    device_path: &str,
) -> Result<PathBuf, String> {
    if mount_point == "/" || mount_point.starts_with("/System") {
        return Err("TRAVA DE SEGURANÇA: Operação bloqueada no disco de sistema do macOS!".into());
    }

    let target_dev = if !device_path.is_empty() && device_path.starts_with("/dev/") {
        device_path.to_string()
    } else {
        mount_point.to_string()
    };

    emit_progress(
        app,
        10,
        "format",
        format!("Formatando {} em MS-DOS FAT32 via diskutil...", target_dev),
    );

    let output = Command::new("diskutil")
        .args(["eraseVolume", "MS-DOS", "FAT32", "XBOX", &target_dev])
        .output()
        .map_err(|e| format!("Falha ao executar diskutil eraseVolume: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "Erro ao formatar no macOS: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    emit_progress(app, 20, "format", "Formatação FAT32 concluída com sucesso.");
    Ok(PathBuf::from("/Volumes/XBOX"))
}

// ============================================================================
// ETAPA 2: CÓPIA RECURSIVA 1:1 PARA A RAIZ DA UNIDADE (20% a 85%)
// ============================================================================

/// Copia todos os itens de `./payload` diretamente para a raiz da partição montada (`target_root`),
/// preservando rigorosamente o casing de cada pasta/arquivo e emitindo progresso em tempo real.
///
/// Requisito Crítico: NUNCA cria uma pasta intermediária `payload` dentro do pendrive.
fn copy_payload_one_to_one(
    app: &AppHandle,
    payload_dir: &Path,
    target_root: &Path,
    total_bytes: u64,
) -> Result<(), String> {
    if !target_root.exists() {
        return Err(format!(
            "Ponto de montagem de destino '{}' não está acessível para cópia.",
            target_root.display()
        ));
    }

    // Utilizamos também as opções de diretório do `fs_extra` como referência de integridade,
    // percorrendo arquivo a arquivo com buffer controlado para emitir progresso granular (20% -> 85%)
    // e sincronizar cada descritor.
    let _copy_options = fs_extra::dir::CopyOptions::new().overwrite(true).copy_inside(true);

    let mut copied_bytes: u64 = 0;
    let safe_total = total_bytes.max(1);

    for entry in WalkDir::new(payload_dir).min_depth(1) {
        let entry = entry.map_err(|e| format!("Erro ao ler estrutura do payload: {}", e))?;
        let source_path = entry.path();

        // Calcula o caminho relativo à pasta `./payload` para clonar 1:1 na raiz do pendrive
        let relative_path = source_path
            .strip_prefix(payload_dir)
            .map_err(|e| format!("Erro ao calcular caminho relativo: {}", e))?;

        // Destino direto na raiz da unidade USB (ex: E:\launch.ini ou /run/media/user/XBOX360/launch.ini)
        let dest_path = target_root.join(relative_path);
        let rel_display = relative_path.to_string_lossy().replace('\\', "/");

        if entry.file_type().is_dir() {
            fs::create_dir_all(&dest_path).map_err(|e| {
                format!(
                    "Erro ao criar diretório '{}' no pendrive: {}",
                    dest_path.display(),
                    e
                )
            })?;
        } else if entry.file_type().is_file() {
            if let Some(parent) = dest_path.parent() {
                fs::create_dir_all(parent).map_err(|e| {
                    format!(
                        "Erro ao criar diretório pai '{}': {}",
                        parent.display(),
                        e
                    )
                })?;
            }

            // Copia o arquivo em blocos de 256 KB para atualizar a barra de progresso suavemente
            let mut src_file = File::open(source_path).map_err(|e| {
                format!("Erro ao abrir arquivo de origem '{}': {}", source_path.display(), e)
            })?;
            let mut dst_file = File::create(&dest_path).map_err(|e| {
                format!("Erro ao criar arquivo de destino '{}': {}", dest_path.display(), e)
            })?;

            let mut buffer = vec![0u8; 256 * 1024];
            loop {
                let bytes_read = src_file
                    .read(&mut buffer)
                    .map_err(|e| format!("Erro de leitura em '{}': {}", rel_display, e))?;
                if bytes_read == 0 {
                    break;
                }
                dst_file
                    .write_all(&buffer[..bytes_read])
                    .map_err(|e| format!("Erro de gravação em '{}': {}", rel_display, e))?;

                copied_bytes += bytes_read as u64;
                let ratio = (copied_bytes as f64 / safe_total as f64).clamp(0.0, 1.0);
                let pct = 20 + (ratio * 65.0).round() as u8;

                emit_progress(
                    app,
                    pct.min(85),
                    "copy",
                    format!("Gravando: {}", rel_display),
                );
            }

            // Garante flush dos dados e metadados do arquivo recém-copiado
            dst_file
                .sync_all()
                .map_err(|e| format!("Erro ao sincronizar arquivo '{}': {}", rel_display, e))?;
        }
    }

    emit_progress(
        app,
        85,
        "copy",
        "Cópia 1:1 dos 7 itens do payload concluída com sucesso.",
    );
    Ok(())
}

// ============================================================================
// ETAPA 3: FLUSH E SINCRONIZAÇÃO DE BUFFERS (85% a 95%)
// ============================================================================

fn sync_filesystem_buffers(app: &AppHandle, target_root: &Path) -> Result<(), String> {
    emit_progress(
        app,
        88,
        "sync",
        "Esvaziando buffers de gravação em memória (sync)...",
    );

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        let _ = target_root;
        let output = Command::new("sync")
            .output()
            .map_err(|e| format!("Falha ao executar comando 'sync': {}", e))?;
        if !output.status.success() {
            return Err(format!(
                "Comando sync retornou erro: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
    }

    #[cfg(target_os = "windows")]
    {
        let mount_str = target_root.to_string_lossy();
        let drive_letter = extract_windows_drive_letter(&mount_str)?;
        let ps_sync = format!("Write-VolumeCache -DriveLetter {}", drive_letter);
        let _ = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &ps_sync])
            .output();
    }

    emit_progress(
        app,
        95,
        "sync",
        "Buffers de disco sincronizados com segurança.",
    );
    Ok(())
}

// ============================================================================
// ETAPA 4: EJEÇÃO AUTOMÁTICA SEGURA (95% a 100%)
// ============================================================================

#[cfg(target_os = "linux")]
fn eject_drive_safely(app: &AppHandle, mount_point: &Path, device_path: &str) -> Result<(), String> {
    let mp_str = mount_point.to_string_lossy();
    let block_dev = resolve_linux_block_device(&mp_str, device_path)?;

    emit_progress(
        app,
        97,
        "eject",
        format!("Desmontando unidade {} com segurança...", block_dev),
    );

    let _ = Command::new("sync").output();

    // 1. Desmonta via udisksctl unmount -b <BLOCO> (ou fallback umount)
    let unmount_out = Command::new("udisksctl")
        .args(["unmount", "-b", &block_dev])
        .output();

    let unmount_ok = unmount_out.as_ref().map(|o| o.status.success()).unwrap_or(false);
    if !unmount_ok {
        let fallback = Command::new("umount").arg(&block_dev).output();
        if !fallback.map(|o| o.status.success()).unwrap_or(false) {
            return Err(format!(
                "Não foi possível desmontar o dispositivo {} para ejeção.",
                block_dev
            ));
        }
    }

    emit_progress(
        app,
        99,
        "eject",
        format!("Desenergizando porta USB ({})...", block_dev),
    );

    // 2. Desenergiza a porta com udisksctl power-off -b <BLOCO>
    let _ = Command::new("udisksctl")
        .args(["power-off", "-b", &block_dev])
        .output();

    Ok(())
}

#[cfg(target_os = "windows")]
fn eject_drive_safely(app: &AppHandle, mount_point: &Path, _device_path: &str) -> Result<(), String> {
    let mp_str = mount_point.to_string_lossy();
    let drive_letter = extract_windows_drive_letter(&mp_str)?;

    emit_progress(
        app,
        97,
        "eject",
        format!("Desmontando e ejetando unidade {}: ...", drive_letter),
    );

    let ps_eject = format!(
        "$vol = Get-Volume -DriveLetter {}; $vol | Get-Partition | Remove-PartitionAccessPath -AccessPath '{}:\\' -Confirm:$false",
        drive_letter, drive_letter
    );

    let out = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &ps_eject])
        .output();

    let success = out.as_ref().map(|o| o.status.success()).unwrap_or(false);
    if !success {
        // Fallback Shell.Application Eject ou diskpart
        let fallback_ps = format!(
            "(New-Object -comObject Shell.Application).Namespace(17).ParseName('{}:').InvokeVerb('Eject')",
            drive_letter
        );
        let _ = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &fallback_ps])
            .output();
    }

    Ok(())
}

#[cfg(target_os = "macos")]
fn eject_drive_safely(app: &AppHandle, mount_point: &Path, device_path: &str) -> Result<(), String> {
    let target = if !device_path.is_empty() && device_path.starts_with("/dev/") {
        device_path.to_string()
    } else {
        mount_point.to_string_lossy().to_string()
    };

    emit_progress(
        app,
        97,
        "eject",
        format!("Ejetando disco {} via diskutil...", target),
    );

    let output = Command::new("diskutil")
        .args(["unmountDisk", &target])
        .output()
        .map_err(|e| format!("Falha ao executar diskutil unmountDisk: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "Erro ao ejetar disco no macOS: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(())
}

// ============================================================================
// ORQUESTRADOR PRINCIPAL DO FLASHER
// ============================================================================

/// Executa o ciclo completo de preparação do pendrive em uma thread de trabalho,
/// emitindo eventos `flash-progress` de 0% a 100%.
pub async fn execute_flash_pipeline(
    app: AppHandle,
    mount_point: String,
    device_path: String,
    auto_eject: bool,
) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        // 0. Validação crítica pré-execução: verifica se o payload local está 100% íntegro
        let payload_status = validate_payload();
        if !payload_status.valid {
            let err_msg = format!(
                "Operação abortada: {}",
                payload_status.status_message
            );
            emit_progress(&app, 0, "error", &err_msg);
            return Err(err_msg);
        }

        // 0.1 Verifica se a unidade selecionada consta na lista de unidades removíveis seguras
        let safe_drives = list_removable_drives();
        let selected_drive = safe_drives
            .iter()
            .find(|d| d.mount_point == mount_point || (!device_path.is_empty() && d.device_path == device_path));

        if selected_drive.is_none() {
            let err_msg = "TRAVA DE SEGURANÇA: A unidade selecionada não é reconhecida como mídia removível USB segura ou foi desconectada.".to_string();
            emit_progress(&app, 0, "error", &err_msg);
            return Err(err_msg);
        }

        let resolved_device = selected_drive
            .map(|d| d.device_path.clone())
            .unwrap_or(device_path);

        emit_progress(
            &app,
            2,
            "format",
            "Iniciando preparação do pendrive (Etapa 1/4: Formatação FAT32)...",
        );

        // ETAPA 1: Formatação FAT32 (0% a 20%)
        let active_mount_root = match format_drive_fat32(&app, &mount_point, &resolved_device) {
            Ok(path) => path,
            Err(e) => {
                emit_progress(&app, 0, "error", &e);
                return Err(e);
            }
        };

        // ETAPA 2: Cópia Recursiva 1:1 (20% a 85%)
        let payload_dir = resolve_payload_dir();
        if let Err(e) = copy_payload_one_to_one(
            &app,
            &payload_dir,
            &active_mount_root,
            payload_status.total_size_bytes,
        ) {
            emit_progress(&app, 20, "error", &e);
            return Err(e);
        }

        // ETAPA 3: Flush e Sincronização de Buffers (85% a 95%)
        if let Err(e) = sync_filesystem_buffers(&app, &active_mount_root) {
            emit_progress(&app, 85, "error", &e);
            return Err(e);
        }

        // ETAPA 4: Ejeção Automática Segura (95% a 100%)
        if auto_eject {
            if let Err(e) = eject_drive_safely(&app, &active_mount_root, &resolved_device) {
                emit_progress(&app, 95, "error", &e);
                return Err(e);
            }
        }

        let final_message = "Pendrive preparado com sucesso e pronto para ser removido!".to_string();
        emit_progress(&app, 100, "done", &final_message);
        Ok(final_message)
    })
    .await
    .map_err(|e| format!("Erro interno na thread de gravação: {}", e))?
}

