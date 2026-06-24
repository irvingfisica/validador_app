import * as d3 from 'd3';
import * as bootstrap from 'bootstrap';

export function limpiarEvento() {
  if (window.appState.colaUnlisten) {
      window.appState.colaUnlisten();
      window.appState.colaUnlisten = null;
  }
}


export function enableTB(boton) {
  d3.selectAll(".tools").classed("active", false);
  d3.selectAll(".tools").attr("aria-pressed", false);
  d3.select(boton).classed("active", true);
  d3.select(boton).attr("aria-pressed", true);
}

const statusBox = d3.select("#status");
const spinner = d3.select("#spinner");

export function setStatus(msg) {
    statusBox.html(msg);
}

export function clearStatus() {
    statusBox.html("");
}

export function showSpinner() {
  spinner.classed("d-none", false);
}

export function hideSpinner() {
  spinner.classed("d-none", true);
}

export function showToast(message, type = "danger") {
  // type: "success", "danger", "warning", "info"

  const container = document.getElementById("toast-container");

  const toastEl = document.createElement("div");
  toastEl.className = `toast align-items-center text-bg-${type} border-0`;
  toastEl.role = "alert";
  toastEl.innerHTML = `
    <div class="d-flex">
      <div class="toast-body">${message}</div>
      <button type="button" class="btn-close btn-close-white me-2 m-auto"
              data-bs-dismiss="toast" aria-label="Close"></button>
    </div>
  `;

  container.appendChild(toastEl);

  const toast = new bootstrap.Toast(toastEl, { delay: 4000 });
  toast.show();

  toastEl.addEventListener("hidden.bs.toast", () => toastEl.remove());
}

export let formato = d3.format(",");