/**
 * XboxLiberator — Lógica de Interface (Vanilla JavaScript + Tauri v2 IPC)
 *
 * Responsabilidades:
 * 1. Consultar e exibir a validação dos 7 itens obrigatórios de `./payload`.
 * 2. Listar somente unidades USB removíveis (`is_removable == true`) vindas do Rust.
 * 3. Exibir modal de confirmação antes da formatação em FAT32.
 * 4. Escutar o evento `flash-progress` em tempo real (0% a 100%).
 */

(function () {
  "use strict";

  // Estado local da aplicação
  const state = {
    payloadValid: false,
    payloadStatus: null,
    drives: [],
    selectedDrive: null,
    isFlashing: false,
  };

  // Referências DOM
  const payloadIndicator = document.getElementById("payload-indicator");
  const payloadSize = document.getElementById("payload-size");
  const payloadChecklist = document.getElementById("payload-checklist");
  const btnRecheckPayload = document.getElementById("btn-recheck-payload");

  const driveSelect = document.getElementById("drive-select");
  const driveDetails = document.getElementById("drive-details");
  const btnRefreshDrives = document.getElementById("btn-refresh-drives");

  const btnFlash = document.getElementById("btn-flash");
  const chkAutoEject = document.getElementById("chk-auto-eject");
  const progressStageLabel = document.getElementById("progress-stage-label");
  const progressPercentage = document.getElementById("progress-percentage");
  const progressBarFill = document.getElementById("progress-bar-fill");
  const progressTrack = document.getElementById("progress-track");
  const progressStatusText = document.getElementById("progress-status-text");

  const confirmModal = document.getElementById("confirm-modal");
  const modalTargetSummary = document.getElementById("modal-target-summary");
  const btnModalCancel = document.getElementById("btn-modal-cancel");
  const btnModalConfirm = document.getElementById("btn-modal-confirm");

  /**
   * Obtém a ponte `invoke` e `listen` do Tauri v2 (`window.__TAURI__`).
   */
  function getTauriApi() {
    if (window.__TAURI__ && window.__TAURI__.core && window.__TAURI__.event) {
      return {
        invoke: window.__TAURI__.core.invoke,
        listen: window.__TAURI__.event.listen,
      };
    }
    return null;
  }

  /**
   * Atualiza o estado de habilitação do botão principal "Montar Pendrive".
   */
  function updateFlashButtonState() {
    const canFlash =
      state.payloadValid &&
      state.selectedDrive !== null &&
      !state.isFlashing;

    btnFlash.disabled = !canFlash;
    btnRecheckPayload.disabled = state.isFlashing;
    btnRefreshDrives.disabled = state.isFlashing;
    driveSelect.disabled = state.isFlashing;
    chkAutoEject.disabled = state.isFlashing;
  }

  /**
   * Atualiza o painel visual de progresso (0% a 100%).
   */
  function setProgressUI(percentage, stageLabel, message, isError = false) {
    const clamped = Math.max(0, Math.min(100, Number(percentage) || 0));
    progressPercentage.textContent = `${clamped}%`;
    progressStageLabel.textContent = stageLabel;
    progressBarFill.style.width = `${clamped}%`;
    progressTrack.setAttribute("aria-valuenow", String(clamped));
    progressStatusText.textContent = message;
    progressStatusText.title = message;

    if (isError) {
      progressBarFill.classList.add("error");
    } else {
      progressBarFill.classList.remove("error");
    }
  }

  /**
   * Valida a pasta `./payload` no backend Rust e atualiza o Card 1.
   */
  async function refreshPayloadStatus() {
    const api = getTauriApi();
    if (!api) {
      payloadIndicator.textContent = "Ambiente Tauri não detectado";
      payloadIndicator.className = "status-indicator invalid";
      return;
    }

    try {
      const status = await api.invoke("check_payload_status");
      state.payloadStatus = status;
      state.payloadValid = Boolean(status.valid);

      payloadSize.textContent = status.formatted_size || "0.00 MB";
      payloadIndicator.textContent = status.status_message;
      payloadIndicator.className = status.valid
        ? "status-indicator valid"
        : "status-indicator invalid";

      // Marca visualmente cada um dos 7 itens obrigatórios
      const missingSet = new Set(status.missing_items || []);
      const items = payloadChecklist.querySelectorAll("li[data-item]");
      items.forEach((li) => {
        const itemName = li.getAttribute("data-item");
        if (missingSet.has(itemName)) {
          li.className = "missing";
        } else {
          li.className = "ok";
        }
      });

      if (!status.valid && !state.isFlashing) {
        setProgressUI(
          0,
          "Payload Incompleto",
          "Payload incompleto ou ausente. Verifique os 7 itens obrigatórios em ./payload.",
          true
        );
      }
    } catch (err) {
      state.payloadValid = false;
      payloadIndicator.textContent = "Payload incompleto ou ausente";
      payloadIndicator.className = "status-indicator invalid";
      console.error("Erro ao validar ./payload:", err);
    } finally {
      updateFlashButtonState();
    }
  }

  /**
   * Consulta as unidades USB removíveis no backend Rust e atualiza o Card 2.
   */
  async function refreshRemovableDrives() {
    const api = getTauriApi();
    if (!api) return;

    const previousMount = state.selectedDrive ? state.selectedDrive.mount_point : "";

    try {
      const drives = await api.invoke("get_removable_drives");
      state.drives = Array.isArray(drives) ? drives : [];

      driveSelect.innerHTML = "";

      if (state.drives.length === 0) {
        const emptyOpt = document.createElement("option");
        emptyOpt.value = "";
        emptyOpt.textContent = "Nenhum pendrive USB detectado";
        driveSelect.appendChild(emptyOpt);

        state.selectedDrive = null;
        driveDetails.textContent =
          "Conecte um pendrive USB e clique em 'Atualizar Lista USB'. Partições internas do sistema estão ocultas.";
      } else {
        const placeholder = document.createElement("option");
        placeholder.value = "";
        placeholder.textContent = "Selecione uma unidade USB...";
        driveSelect.appendChild(placeholder);

        state.drives.forEach((drive, idx) => {
          const opt = document.createElement("option");
          opt.value = String(idx);
          // Formato obrigatório: "Nome do Volume (Ponto de Montagem) - Tamanho em GB"
          opt.textContent = drive.display_label;
          driveSelect.appendChild(opt);
        });

        // Restaura seleção anterior se o mesmo pendrive continuar conectado, ou seleciona o primeiro se houver apenas 1
        const restoredIndex = state.drives.findIndex(
          (d) => d.mount_point === previousMount
        );
        if (restoredIndex >= 0) {
          driveSelect.value = String(restoredIndex);
          state.selectedDrive = state.drives[restoredIndex];
        } else if (state.drives.length === 1) {
          driveSelect.value = "0";
          state.selectedDrive = state.drives[0];
        } else {
          state.selectedDrive = null;
        }

        renderSelectedDriveDetails();
      }
    } catch (err) {
      console.error("Erro ao listar unidades removíveis:", err);
      driveDetails.textContent = `Erro ao consultar dispositivos USB: ${err}`;
    } finally {
      updateFlashButtonState();
    }
  }

  /**
   * Exibe os metadados da unidade USB selecionada no Card 2.
   */
  function renderSelectedDriveDetails() {
    if (!state.selectedDrive) {
      driveDetails.textContent =
        "Selecione o pendrive de destino no menu acima para preparar o pacote Xbox 360.";
      return;
    }

    const d = state.selectedDrive;
    driveDetails.innerHTML = `
      <div><strong>Volume:</strong> ${escapeHtml(d.name)}</div>
      <div><strong>Montagem:</strong> <code>${escapeHtml(d.mount_point)}</code></div>
      <div><strong>Dispositivo:</strong> <code>${escapeHtml(d.device_path)}</code></div>
      <div><strong>Capacidade:</strong> ${escapeHtml(d.formatted_size_gb)} (Atual: ${escapeHtml(d.file_system || "Desconhecido")})</div>
    `;

    if (!state.isFlashing && state.payloadValid) {
      setProgressUI(
        0,
        "Pronto para gravar",
        `Destino selecionado: ${d.display_label}`
      );
    }
  }

  function escapeHtml(str) {
    return String(str)
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;");
  }

  /**
   * Abre o modal de confirmação obrigatório antes de formatar a unidade.
   */
  function openConfirmModal() {
    if (!state.selectedDrive || !state.payloadValid || state.isFlashing) return;

    modalTargetSummary.innerHTML = `
      <strong>Unidade selecionada:</strong><br/>
      ${escapeHtml(state.selectedDrive.display_label)}
    `;
    confirmModal.classList.remove("hidden");
  }

  function closeConfirmModal() {
    confirmModal.classList.add("hidden");
  }

  /**
   * Dispara a execução sequencial das 4 etapas no backend Rust.
   */
  async function startFlashing() {
    closeConfirmModal();

    const api = getTauriApi();
    if (!api || !state.selectedDrive) return;

    state.isFlashing = true;
    updateFlashButtonState();

    const targetDrive = state.selectedDrive;
    const autoEject = Boolean(chkAutoEject.checked);

    setProgressUI(
      1,
      "Iniciando...",
      `Preparando formatação FAT32 em ${targetDrive.mount_point}...`
    );

    try {
      const successMsg = await api.invoke("flash_usb_drive", {
        mountPoint: targetDrive.mount_point,
        devicePath: targetDrive.device_path,
        autoEject: autoEject,
      });

      setProgressUI(100, "Concluído!", successMsg, false);
      // Atualiza lista de drives (caso tenha sido ejetado, ele sairá da lista)
      await refreshRemovableDrives();
    } catch (err) {
      const errorText = typeof err === "string" ? err : JSON.stringify(err);
      setProgressUI(0, "Falha na operação", errorText, true);
    } finally {
      state.isFlashing = false;
      updateFlashButtonState();
    }
  }

  /**
   * Registra o listener do evento `flash-progress` emitido pelo backend Rust.
   */
  async function setupProgressListener() {
    const api = getTauriApi();
    if (!api) return;

    const stageNames = {
      format: "Etapa 1/4: Formatação FAT32",
      copy: "Etapa 2/4: Cópia 1:1 do Payload",
      sync: "Etapa 3/4: Sincronizando Buffers (Sync)",
      eject: "Etapa 4/4: Ejeção Segura",
      done: "Concluído com Sucesso!",
      error: "Erro na Gravação",
    };

    await api.listen("flash-progress", (event) => {
      const data = event.payload || {};
      const label = stageNames[data.stage] || "Processando...";
      const isErr = data.stage === "error";
      setProgressUI(data.percentage, label, data.message || "", isErr);
    });
  }

  // Eventos de UI
  btnRecheckPayload.addEventListener("click", () => {
    refreshPayloadStatus();
  });

  btnRefreshDrives.addEventListener("click", () => {
    refreshRemovableDrives();
  });

  driveSelect.addEventListener("change", (e) => {
    const idx = e.target.value === "" ? -1 : Number(e.target.value);
    state.selectedDrive =
      idx >= 0 && idx < state.drives.length ? state.drives[idx] : null;
    renderSelectedDriveDetails();
    updateFlashButtonState();
  });

  btnFlash.addEventListener("click", () => {
    openConfirmModal();
  });

  btnModalCancel.addEventListener("click", () => {
    closeConfirmModal();
  });

  btnModalConfirm.addEventListener("click", () => {
    startFlashing();
  });

  // Inicialização ao carregar o DOM
  window.addEventListener("DOMContentLoaded", async () => {
    await setupProgressListener();
    await refreshPayloadStatus();
    await refreshRemovableDrives();
  });
})();

