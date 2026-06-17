const { invoke } = window.__TAURI__.core;
import { AllCommunityModule, ModuleRegistry, createGrid } from 'ag-grid-community';


import * as d3 from 'd3';
import * as utils from "./utils.js";

export async function intface() {
    utils.enableTB("#subirTool");
    
    const contenedor = d3.select("#mesaTrabajo");
    contenedor.selectAll("*").remove();

    const desc = contenedor.append("div").attr("class", "col-md-12");

    desc.append("h1").html("Herramienta de subida");
    desc.append("p")
        .html(
        "La herramienta permite subir el archivo al repositorio y a la PNDA.",
        );

        let instituciones = await invoke("obtener_instituciones");
        console.log(instituciones);

}