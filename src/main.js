import { save } from '@tauri-apps/plugin-dialog';
import { getCurrentWindow } from '@tauri-apps/api/window';
import 'bootstrap/dist/css/bootstrap.min.css';
import 'bootstrap/dist/js/bootstrap.bundle.min.js';
import * as d3 from 'd3';

import * as carga from "./tools/carga.js";
import * as columns from "./tools/columns.js";
import * as categos from "./tools/categos.js";

const { invoke } = window.__TAURI__.core;

window.appState = {
  grid: null,
  file: null,
};
window.otherGrid = null;
window.dropUnlisten = null;
window.procesando = false;

const appWindow = getCurrentWindow();

appWindow.onDragDropEvent(async (event) => {

    if (!document.querySelector("#dropZone")) {
        return;
    }

    await carga.procesarDrop(event);
})

d3.select("#cargarTool").on("click", carga.intface);
d3.select("#validacionTool").on("click", columns.intface);
d3.select("#categosTool").on("click", categos.intface);

carga.intface();

d3.select("#descargaTool")
    .on("click", async () => {

        const nombre = await invoke("ruta_sugerida");

        const ruta = await save({
            title: "Guardar CSV",
            defaultPath: nombre,
            filters: [
                {
                    name: "CSV",
                    extensions: ["csv"]
                }
            ]
        });

        if (!ruta) {
            return;
        }

        await invoke("exportar_csv", { ruta });

        alert("Archivo exportado");
    });