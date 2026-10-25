# 🎮 XboxLiberator

**Prepare o seu pendrive de Xbox 360 em 1 clique** — Formatação automática em FAT32, cópia cirúrgica 1:1 do pacote de destrava (`./payload`), sincronização de cache e ejeção segura.

> **✨ 100% Pronto para Uso:** Você **não** precisa saber programar nem compilar nada! Basta clicar no botão do seu sistema operacional abaixo, baixar e abrir.

---

## ⬇️ Download Direto (Instaladores Prontos)

Escolha o seu sistema operacional abaixo para baixar a versão mais recente diretamente:

| Sistema Operacional | Formato | Link Direto de Download | Como Usar |
| :--- | :--- | :--- | :--- |
| 🪟 **Windows 10 / 11** | Instalador (`.exe`) | [**📥 Baixar Instalador Windows (.exe)**](https://github.com/EnzoGiachetti/XboxLiberator/releases/latest) | Dê dois cliques no `-setup.exe` e abra o **XboxLiberator**. |
| 🪟 **Windows (Portátil)** | Arquivo `.zip` | [**📥 Baixar Windows Portátil (.zip)**](https://github.com/EnzoGiachetti/XboxLiberator/releases/latest) | Extraia o `.zip` e dê dois cliques em `XboxLiberator.exe`. |
| 🐧 **Fedora / RHEL / openSUSE** | Pacote `.rpm` | [**📥 Baixar para Fedora (.rpm)**](https://github.com/EnzoGiachetti/XboxLiberator/releases/latest) | Dê dois cliques no `.rpm` ou rode `sudo dnf install ./XboxLiberator*.rpm`. |
| 🐧 **Ubuntu / Debian / Mint** | Pacote `.deb` | [**📥 Baixar para Ubuntu/Mint (.deb)**](https://github.com/EnzoGiachetti/XboxLiberator/releases/latest) | Dê dois cliques no `.deb` ou rode `sudo apt install ./XboxLiberator*.deb`. |
| 🐧 **Qualquer Linux (Portátil)** | `.AppImage` | [**📥 Baixar Linux Universal (.AppImage)**](https://github.com/EnzoGiachetti/XboxLiberator/releases/latest) | Marque como executável (`chmod +x *.AppImage`) e dê dois cliques. |
| 🍎 **macOS (Apple Silicon & Intel)** | Imagem `.dmg` | [**📥 Baixar para macOS (.dmg)**](https://github.com/EnzoGiachetti/XboxLiberator/releases/latest) | Abra o `.dmg` e arraste o **XboxLiberator** para `Aplicativos`. |

👉 **[Clique aqui para ver todos os arquivos na página oficial de Releases](https://github.com/EnzoGiachetti/XboxLiberator/releases/latest)**

---

## 🚀 Como Usar o Programa (3 Passos Simples)

1. **Conecte o seu Pendrive USB** no computador.
2. **Abra o XboxLiberator**:
   - No **Card 1 (Pacote de Destrava)**, o programa já traz embutido e validado todo o pacote necessário (`Aurora`, `BadUpdatePayload`, `Content`, `Simple 360 NAND Flasher`, `JRPC2.xex`, `launch.ini`, `Xbdm.xex`).
   - No **Card 2 (Unidade de Destino)**, selecione o seu pendrive na lista (discos internos do seu computador ficam ocultos automaticamente para sua total segurança).
3. **Clique em "Montar Pendrive"** no **Card 3**:
   - Confirme o aviso de formatação.
   - Aguarde a barra chegar em **100%** (o programa formata em FAT32, copia os arquivos direto para a raiz do pendrive e ejeta com segurança).
   - **Pronto!** É só remover o pendrive e conectar no Xbox 360.

---

## 📦 Estrutura Interna do Payload Incluído

O pacote embutido em `./payload` é copiado na proporção **1:1 diretamente para a raiz do pendrive** (sem criar pastas intermediárias):

```text
Raiz do Pendrive (USB)/
├── Aurora/
├── BadUpdatePayload/
├── Content/
├── Simple 360 NAND Flasher/
├── JRPC2.xex
├── launch.ini
└── Xbdm.xex
```

---

## 🛠️ Área para Desenvolvedores (Compilação Manual Opcional)

<details>
<summary><strong>Clique aqui se você for desenvolvedor e quiser compilar o código-fonte manualmente</strong></summary>

### 1. Pré-requisitos
- **Rust** (`rustup`, `cargo`, `rustc` estável)
- **Tauri CLI v2**:
  ```bash
  cargo install tauri-cli --version "^2.0.0" --locked
  ```

#### Dependências no Fedora / RHEL:
```bash
sudo dnf install -y webkit2gtk4.1-devel gtk3-devel glib2-devel openssl-devel curl wget file libappindicator-gtk3-devel librsvg2-devel dosfstools rpm-build
```

#### Dependências no Ubuntu / Debian:
```bash
sudo apt update
sudo apt install -y libwebkit2gtk-4.1-dev build-essential curl wget file libssl-dev libayatana-appindicator3-dev librsvg2-dev dosfstools rpm
```

### 2. Executar em Modo de Desenvolvimento
```bash
cargo run --manifest-path src-tauri/Cargo.toml
```

### 3. Gerar Instaladores Localmente
```bash
cargo tauri build
```
Os pacotes serão gerados em `src-tauri/target/release/bundle/`.
</details>

---

## ⚖️ Aviso Legal (Disclaimer Educacional)

> **AVISO IMPORTANTE:**  
> O **XboxLiberator** é uma ferramenta open source desenvolvida exclusivamente para **fins educacionais, preservação de hardware, interoperabilidade e execução de software homebrew legítimo** em consoles pertencentes ao próprio usuário.  
> Este projeto **não** distribui código proprietário protegido por direitos autorais de terceiros nem incentiva a pirataria. O uso desta ferramenta e eventuais modificações realizadas em seu hardware ou mídias de armazenamento são de inteira e exclusiva responsabilidade do usuário final.
