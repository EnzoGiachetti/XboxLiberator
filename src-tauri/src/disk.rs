//! Módulo de inspeção de discos removíveis e validação da integridade da pasta `./payload`.
//!
//! Implementa travas de segurança estritas para garantir que apenas dispositivos USB
//! removíveis sejam listados, bloqueando partições de sistema operacional (`/` no Linux/macOS,
//! `C:\` no Windows, etc.) e validando os 7 itens obrigatórios do payload do Xbox 360.

use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use sysinfo::Disks;
use walkdir::WalkDir;

/// Os 7 itens obrigatórios (com casing exato) que devem existir na raiz de `./payload`.
pub const REQUIRED_PAYLOAD_ITEMS: [(&str, bool); 7] = [
    ("Aurora", true),                    // Pasta
    ("BadUpdatePayload", true),          // Pasta
    ("Content", true),                   // Pasta
    ("Simple 360 NAND Flasher", true),   // Pasta
    ("JRPC2.xex", false),                // Arquivo
    ("launch.ini", false),               // Arquivo
    ("Xbdm.xex", false),                 // Arquivo
];

/// Status detalhado da validação da pasta `./payload`.
#[derive(Debug, Clone, Serialize)]
pub struct PayloadStatus {
    /// Indica se todos os 7 itens obrigatórios estão presentes com o casing exato.
    pub valid: bool,
    /// Caminho resolvido da pasta `./payload`.
    pub path: String,
    /// Tamanho total somado em bytes de todos os arquivos dentro de `./payload`.
    pub total_size_bytes: u64,
    /// Tamanho formatado de forma legível (ex: "145.20 MB" ou "1.24 GB").
    pub formatted_size: String,
    /// Quantidade total de arquivos contidos recursivamente no payload.
    pub total_files: usize,
    /// Lista de itens ausentes ou com tipo/casing incorreto (vazia quando `valid == true`).
    pub missing_items: Vec<String>,
    /// Mensagem amigável de status para exibição na interface.
    pub status_message: String,
}

/// Representa uma unidade USB removível segura para formatação e gravação.
#[derive(Debug, Clone, Serialize)]
pub struct RemovableDrive {
    /// Nome ou rótulo do volume (ex: "KINGSTON" ou "USB Drive").
    pub name: String,
    /// Ponto de montagem atual no sistema (ex: `/run/media/user/USB` ou `E:\`).
    pub mount_point: String,
    /// Caminho do dispositivo de bloco ou identificador de unidade (ex: `/dev/sdb1`, `E:`).
    pub device_path: String,
    /// Sistema de arquivos atual (ex: "FAT32", "exFAT", "NTFS").
    pub file_system: String,
    /// Capacidade total em bytes.
    pub total_space_bytes: u64,
    /// Capacidade total formatada em GB (ex: "14.9 GB").
    pub formatted_size_gb: String,
    /// Rótulo formatado no padrão exigido pela UI: "Nome do Volume (Ponto de Montagem) - Tamanho em GB".
    pub display_label: String,
}

/// Localiza o diretório `./payload` buscando a partir do diretório de trabalho atual,
/// do diretório pai (quando executado dentro de `src-tauri`) ou ao lado do executável.
pub fn resolve_payload_dir() -> PathBuf {
    let candidates = [
        PathBuf::from("./payload"),
        PathBuf::from("../payload"),
    ];

    for candidate in &candidates {
        if candidate.exists() && candidate.is_dir() {
            return candidate.canonicalize().unwrap_or_else(|_| candidate.clone());
        }
    }

    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let beside_exe = exe_dir.join("payload");
            if beside_exe.exists() && beside_exe.is_dir() {
                return beside_exe;
            }
            // Suporte a recursos empacotados pelo Tauri (Linux /usr/lib/..., macOS Resources)
            let up_one = exe_dir.join("../Resources/payload");
            if up_one.exists() && up_one.is_dir() {
                return up_one;
            }
        }
    }

    PathBuf::from("./payload")
}

/// Verifica se uma entrada existe dentro de `dir` com correspondência exata de maiúsculas
/// e minúsculas (case-sensitive mesmo em sistemas de arquivos case-insensitive como Windows/macOS).
fn exists_with_exact_case(dir: &Path, expected_name: &str, expect_dir: bool) -> bool {
    let Ok(entries) = fs::read_dir(dir) else {
        return false;
    };

    for entry in entries.flatten() {
        if entry.file_name().to_string_lossy() == expected_name {
            if let Ok(file_type) = entry.file_type() {
                return if expect_dir {
                    file_type.is_dir()
                } else {
                    file_type.is_file()
                };
            }
        }
    }
    false
}

/// Formata um valor em bytes para MB ou GB com duas casas decimais.
pub fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;

    let b = bytes as f64;
    if b >= GB {
        format!("{:.2} GB", b / GB)
    } else if b >= MB {
        format!("{:.2} MB", b / MB)
    } else if b >= KB {
        format!("{:.2} KB", b / KB)
    } else {
        format!("{} B", bytes)
    }
}

/// Valida se a pasta `./payload` existe e contém exatamente os 7 itens obrigatórios
/// com seus respectivos tipos (4 pastas e 3 arquivos) e casing preservado.
pub fn validate_payload() -> PayloadStatus {
    let payload_path = resolve_payload_dir();
    let display_path = payload_path.to_string_lossy().to_string();

    if !payload_path.exists() || !payload_path.is_dir() {
        return PayloadStatus {
            valid: false,
            path: display_path,
            total_size_bytes: 0,
            formatted_size: "0.00 MB".to_string(),
            total_files: 0,
            missing_items: REQUIRED_PAYLOAD_ITEMS
                .iter()
                .map(|(name, _)| (*name).to_string())
                .collect(),
            status_message: "Payload incompleto ou ausente (pasta ./payload não encontrada)".to_string(),
        };
    }

    let mut missing_items = Vec::new();
    for (item_name, is_dir) in REQUIRED_PAYLOAD_ITEMS {
        if !exists_with_exact_case(&payload_path, item_name, is_dir) {
            missing_items.push(item_name.to_string());
        }
    }

    // Calcula o tamanho total e o número de arquivos recursivamente
    let mut total_size_bytes: u64 = 0;
    let mut total_files: usize = 0;

    for entry in WalkDir::new(&payload_path).into_iter().flatten() {
        if entry.file_type().is_file() {
            total_files += 1;
            if let Ok(meta) = entry.metadata() {
                total_size_bytes += meta.len();
            }
        }
    }

    let valid = missing_items.is_empty();
    let status_message = if valid {
        format!("Payload íntegro (7/7 itens verificados • {} arquivos)", total_files)
    } else {
        format!(
            "Payload incompleto ou ausente (faltando: {})",
            missing_items.join(", ")
        )
    };

    PayloadStatus {
        valid,
        path: display_path,
        total_size_bytes,
        formatted_size: format_bytes(total_size_bytes),
        total_files,
        missing_items,
        status_message,
    }
}

/// Verifica se um ponto de montagem ou dispositivo pertence ao sistema operacional
/// e deve ser estritamente bloqueado (Trava de Segurança Crítica).
fn is_protected_system_mount(mount_point: &str, device_name: &str) -> bool {
    let mp_trimmed = mount_point.trim().trim_end_matches(['/', '\\']);
    let mp_upper = mp_trimmed.to_uppercase();

    // Pontos de montagem raiz ou diretórios críticos Unix/Linux/macOS
    let forbidden_unix_mounts = [
        "",
        "/",
        "/boot",
        "/boot/efi",
        "/home",
        "/root",
        "/usr",
        "/var",
        "/opt",
        "/etc",
        "/tmp",
        "/srv",
        "/proc",
        "/sys",
        "/dev",
        "/run",
        "/System",
        "/System/Volumes/Data",
        "/System/Volumes/Preboot",
        "/System/Volumes/VM",
        "/System/Volumes/Update",
        "/Library",
        "/Applications",
        "/Users",
        "/Volumes/Macintosh HD",
    ];

    if mount_point == "/" || forbidden_unix_mounts.contains(&mp_trimmed) {
        return true;
    }

    // Bloqueia submontagens de sistema no Linux (/boot/*, /snap/*, /var/*)
    if mp_trimmed.starts_with("/boot/")
        || mp_trimmed.starts_with("/snap/")
        || mp_trimmed.starts_with("/var/")
        || mp_trimmed.starts_with("/sys/")
        || mp_trimmed.starts_with("/proc/")
        || mp_trimmed.starts_with("/System/")
    {
        return true;
    }

    // Bloqueia unidades de sistema Windows (C:, C:\, e partições de recuperação)
    if mp_upper == "C:" || mp_upper.starts_with("C:\\") || mp_upper.starts_with("C:/") {
        return true;
    }

    // Bloqueia dispositivos NVMe, SATA/ virtuais internos, loopbacks, zram, dm/lvm e mdraid
    let dev_lower = device_name.to_lowercase();
    if dev_lower.contains("nvme")
        || dev_lower.starts_with("/dev/loop")
        || dev_lower.starts_with("/dev/zram")
        || dev_lower.starts_with("/dev/dm-")
        || dev_lower.starts_with("/dev/mapper/")
        || dev_lower.starts_with("/dev/md")
        || dev_lower.starts_with("/dev/ram")
        || dev_lower.starts_with("/dev/sr")
    {
        return true;
    }

    false
}

/// No Linux, faz uma verificação adicional em `/sys/block/<disk>/removable` e no barramento USB
/// para garantir que o dispositivo listado pelo `sysinfo` seja genuinamente removível/USB.
#[cfg(target_os = "linux")]
fn is_linux_usb_or_removable(device_path: &str) -> bool {
    // Exemplo: "/dev/sdb1" -> base "sdb"
    let base_name = device_path
        .trim_start_matches("/dev/")
        .trim_end_matches(|c: char| c.is_ascii_digit());

    if base_name.is_empty() {
        return true;
    }

    let sys_removable = format!("/sys/block/{}/removable", base_name);
    if let Ok(val) = fs::read_to_string(&sys_removable) {
        if val.trim() == "1" {
            return true;
        }
    }

    // Alguns adaptadores USB-SATA ou cases externos reportam removable=0, mas estão no barramento USB
    let sys_device_link = format!("/sys/block/{}", base_name);
    if let Ok(canonical) = fs::canonicalize(&sys_device_link) {
        let canonical_str = canonical.to_string_lossy().to_lowercase();
        if canonical_str.contains("/usb") {
            return true;
        }
    }

    false
}

/// Enumera unicamente as unidades removíveis USB conectadas ao sistema,
/// filtrando rigorosamente qualquer disco interno ou partição do sistema operacional.
pub fn list_removable_drives() -> Vec<RemovableDrive> {
    let disks = Disks::new_with_refreshed_list();
    let mut removable_drives = Vec::new();

    for disk in disks.list() {
        // 1. Trava primária obrigatória: apenas discos marcados como removíveis pelo SO
        if !disk.is_removable() {
            continue;
        }

        let mount_point = disk.mount_point().to_string_lossy().to_string();
        let device_path = disk.name().to_string_lossy().to_string();

        // 2. Trava de segurança crítica contra partições do sistema ("/", "C:", "/home", NVMe, etc.)
        if is_protected_system_mount(&mount_point, &device_path) {
            continue;
        }

        // 3. Verificação extra no Linux via sysfs (/sys/block/*/removable ou barramento USB)
        #[cfg(target_os = "linux")]
        if !is_linux_usb_or_removable(&device_path) {
            continue;
        }

        let total_space_bytes = disk.total_space();
        if total_space_bytes == 0 {
            continue;
        }

        let size_gb = (total_space_bytes as f64) / (1024.0 * 1024.0 * 1024.0);
        let formatted_size_gb = format!("{:.1} GB", size_gb);

        // Determina um nome de volume legível
        let raw_name = disk.name().to_string_lossy().trim().to_string();
        let volume_name = if raw_name.is_empty() || raw_name.starts_with("/dev/") {
            // Tenta extrair o último segmento do ponto de montagem (ex: /run/media/user/XBOX -> XBOX)
            Path::new(&mount_point)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "USB Removível".to_string())
        } else {
            raw_name
        };

        let file_system = disk.file_system().to_string_lossy().to_string();

        // Formato exigido na especificação: "Nome do Volume (Ponto de Montagem) - Tamanho em GB"
        let display_label = format!("{} ({}) - {}", volume_name, mount_point, formatted_size_gb);

        removable_drives.push(RemovableDrive {
            name: volume_name,
            mount_point,
            device_path,
            file_system,
            total_space_bytes,
            formatted_size_gb,
            display_label,
        });
    }

    removable_drives
}

