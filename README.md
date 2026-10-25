# XboxLiberator

**XboxLiberator** é uma aplicação desktop multiplataforma e open source inspirada na simplicidade e segurança do *BalenaEtcher*, projetada especificamente para preparar unidades USB (pendrives) para **Xbox 360** de forma automatizada.

O utilitário realiza em um único fluxo:
1. **Validação estrita** dos 7 itens obrigatórios na pasta local `./payload` (preservando maiúsculas e minúsculas).
2. **Formatação em FAT32** da unidade USB selecionada (`0% -> 20%`).
3. **Clonagem Direta 1:1 para a raiz** da partição USB sem criar pasta intermediária `payload/` (`20% -> 85%`).
4. **Flush e Sincronização de Buffers (`sync`)** para evitar corrupção por cache pendente (`85% -> 95%`).
5. **Ejeção Automática Segura** e desenergização da porta USB (`95% -> 100%`).

---

## Estrutura Obrigatória do Payload (`./payload`)

Na raiz do projeto deve existir o diretório `./payload` contendo exatamente os 7 itens abaixo (com *casing* idêntico):

```text
payload/
├── Aurora/
├── BadUpdatePayload/
├── Content/
├── Simple 360 NAND Flasher/
├── JRPC2.xex
├── launch.ini
└── Xbdm.xex
```

Durante a gravação, os 7 itens são copiados diretamente para a raiz da unidade USB (ex.: `E:\launch.ini` no Windows ou `/run/media/usuario/XBOX360/launch.ini` no Linux).

---

## Instalação dos Pacotes Pré-Compilados

### Linux — Fedora, RHEL e openSUSE (`.rpm`)
```bash
sudo dnf install ./xboxliberator.rpm
cargo run --manifest-path src-tauri/Cargo.toml
```

### Linux — Ubuntu, Debian e Linux Mint (`.deb`)
```bash
sudo apt install ./xboxliberator.deb
```

### Linux — Portátil Universal (`.AppImage`)
```bash
chmod +x ./xboxliberator.AppImage
./xboxliberator.AppImage
```

### Windows (`.exe` NSIS ou Standalone)
Execute o instalador gerado `XboxLiberator_1.0.0_x64-setup.exe` ou o executável portátil `xboxliberator.exe`.

### macOS (`.dmg` Apple Silicon & Intel)
Abra o arquivo `XboxLiberator_1.0.0_aarch64.dmg` (ou `x64.dmg`) e arraste o **XboxLiberator.app** para a pasta `Applications`.

---

## Como Compilar Localmente

### 1. Pré-requisitos
- **Rust** (`rustup`, `cargo`, `rustc` estável)
- **Tauri CLI v2**:
  ```bash
  cargo install tauri-cli --version "^2.0.0"
  ```

#### Dependências de sistema no Fedora / RHEL:
```bash
sudo dnf install webkit2gtk4.1-devel openssl-devel curl wget file libappindicator-gtk3-devel librsvg2-devel dosfstools rpm-build
```

#### Dependências de sistema no Ubuntu / Debian:
```bash
sudo apt update
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file libssl-dev libayatana-appindicator3-dev librsvg2-dev dosfstools rpm
```

### 2. Executar em Modo de Desenvolvimento
```bash
cargo tauri dev
```

### 3. Gerar Pacotes de Produção (`.rpm`, `.deb`, `.AppImage`, `.exe`, `.dmg`)
```bash
cargo tauri build
```
Os instaladores finais serão gerados em `src-tauri/target/release/bundle/`.

---

## Aviso Legal (Disclaimer Educacional)

> **AVISO IMPORTANTE:**  
> O **XboxLiberator** é uma ferramenta open source desenvolvida exclusivamente para **fins educacionais, preservação de hardware, interoperabilidade e execução de software homebrew legítimo** em consoles pertencentes ao próprio usuário.  
> Este projeto **não** distribui código proprietário protegido por direitos autorais de terceiros nem incentiva a pirataria. O uso desta ferramenta e eventuais modificações realizadas em seu hardware ou mídias de armazenamento são de inteira e exclusiva responsabilidade do usuário final.
