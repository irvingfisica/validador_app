
import { getCurrentWindow } from '@tauri-apps/api/window';
const { invoke } = window.__TAURI__.core;

import * as d3 from 'd3';
import * as utils from "./utils.js";
import * as grid from "./grid.js";

export function intface() {
  utils.enableTB("#cargarTool");

  const contenedor = d3.select("#mesaTrabajo");
  contenedor.selectAll("*").remove();

  const desc = contenedor.append("div").attr("class", "col-md-12");

  desc.append("h1").html("Herramientas de limpieza para CSV");
  desc.append("h2").html("¿Cómo funciona esta herramienta?");
  desc
    .append("p")
    .html(
      "Aplica las transformaciones que necesites para que tu base de datos esté más limpia.",
    );
  desc.append("p").html("Comienza cargando un archivo.");

  const dropd = contenedor.append("div").attr("class", "row");

  const drop = dropd
    .append("div")
    .attr("id", "dropZone")
    .attr("class", "drop-zone col-md-12");
  drop.append("p").html("Arrastra un CSV.");

  dropd.append("div")
        .attr("id","errorZone").attr("class","col-md-12 mt-3");

  const framec = dropd
    .append("div")
    .attr("id", "gridBlock")
    .attr("class", "col-md-12 bloque mt-5");

const appWindow = getCurrentWindow();

appWindow.onDragDropEvent(async (event) => {
  if (event.payload.type === 'hover') {
    d3.select("#dropZone").classed("dragover", true);
  } else if (event.payload.type === 'drop') {
    d3.select("#dropZone").classed("dragover", false);
    
    const rutaAbsoluta = event.payload.paths[0];
    window.appState.file = rutaAbsoluta;

    if (!rutaAbsoluta.toLowerCase().endsWith('.csv')) {
      utils.showToast("El archivo debe tener formato CSV.", "danger");
      return;
    };

    if (window.appState.grid) {
        try {
            window.appState.grid.destroy();
        } catch (e) {
          console.warn("Error al intentar destruir el grid anterior:", e);
        }
        window.appState.grid = null;
    }

    utils.setStatus("Analizando codificación e indizando datos...");
    utils.showSpinner();

    try {
        const reporte = await invoke("leer_csv", { ruta: rutaAbsoluta });

        utils.hideSpinner();
        utils.setStatus(`Listo: ${utils.formato(reporte.total_filas)} filas; ${utils.formato(reporte.columnas.length)} columnas.`);

        Object.assign(window.appState, reporte);

        if (window.appState.caracteres_corruptos.length > 0) {
            const enczone = d3.select("#errorZone");

            enczone.append("strong").html("Se detectaron caracteres extraños (es necesario revisar el archivo): ");
            enczone.append("span").attr("class","redc")
                .html(window.appState.caracteres_corruptos.reduce((a,b) => a + b.caracter + ", ", " "));
        }

        const muestra = await invoke("obtener_bloque",{startRow: 0, pageSize: 10});
        console.log(muestra);

        grid.mostrarGrid("#gridBlock");

        d3.select("#dropZone p").html(
            `Archivo actual: <strong>${window.appState.file}</strong>`,
        );

        d3.select("#validacionTool").property("disabled", false);
        d3.select("#incidenciasTool").property("disabled", false);
        d3.select("#categosTool").property("disabled", false);
        d3.select("#descargaTool").property("disabled", false);
    } catch (error) {
        utils.hideSpinner();
        utils.showToast(`No se pudo procesar el archivo. Motivo: ${error}`,"danger");
    }

  } else {
    d3.select("#dropZone").classed("dragover", false);
  }
});

  if (window.appState.grid) {
    d3.select("#dropZone p").html(
      `Archivo actual: <strong>${window.appState.file}</strong>`,
    );

    if (window.appState.caracteres_corruptos.length > 0) {
            const enczone = d3.select("#errorZone");

            enczone.append("strong").html("Se detectaron caracteres extraños (es necesario revisar el archivo): ");
            enczone.attr("class","redc")
                .html(window.appState.caracteres_corruptos.reduce((a,b) => a + b.caracter + ", ", " "));
        }

    d3.select("#validacionTool").property("disabled", false);
    d3.select("#incidenciasTool").property("disabled", false);
    d3.select("#categosTool").property("disabled", false);

    grid.mostrarGrid("#gridBlock");
  } 
}