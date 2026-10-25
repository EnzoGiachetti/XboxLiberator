// Previne a abertura de uma janela de console extra no Windows em modo release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod disk;
mod flasher;

use disk::{PayloadStatus, RemovableDrive};
use tauri::AppHandle;

/// Comando Tauri: Valida a pasta `./payload` verificando a presença e o casing exato
/// dos 7 itens obrigatórios (`Aurora/`, `BadUpdatePayload/`, `Content/`,
/// `Simple 360 NAND Flasher/`, `JRPC2.xex`, `launch.ini`, `Xbdm.xex`) e calcula o tamanho total.
#[tauri::command]
fn check_payload_status() -> PayloadStatus {
    disk::validate_payload()
}

/// Comando Tauri: Lista unicamente mídias USB removíveis (`is_removable == true`),
/// filtrando discos internos SATA/NVMe e diretórios de sistema operacional.
#[tauri::command]
fn get_removable_drives() -> Vec<RemovableDrive> {
    disk::list_removable_drives()
}

/// Comando Tauri: Inicia o fluxo completo de gravação em 4 etapas:
/// 1. Formatação FAT32 (0% - 20%)
/// 2. Cópia Recursiva 1:1 para a raiz do pendrive (20% - 85%)
/// 3. Flush e Sincronização de Buffers `sync` (85% - 95%)
/// 4. Ejeção Segura da unidade USB (95% - 100%)
#[tauri::command]
async fn flash_usb_drive(
    app: AppHandle,
    mount_point: String,
    device_path: String,
    auto_eject: bool,
) -> Result<String, String> {
    flasher::execute_flash_pipeline(app, mount_point, device_path, auto_eject).await
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            check_payload_status,
            get_removable_drives,
            flash_usb_drive
        ])
        .run(tauri::generate_context!())
        .expect("Erro crítico ao inicializar a aplicação Tauri XboxLiberator");
}

