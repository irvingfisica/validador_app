const { invoke } = window.__TAURI__.core;

import * as d3 from 'd3';
import * as utils from "./utils.js";
import * as grid from "./grid.js";

export async function intface() {
    utils.limpiarEvento();
    utils.enableTB("#validacionTool");

    const contenedor = d3.select("#mesaTrabajo");
    contenedor.selectAll("*").remove();

    const desc = contenedor.append("div").attr("class", "col-12");

    desc.append("h1").html("Herramienta de columnas");
    desc.append("p")
        .html(
        "La herramienta sugiere nombres de columna que puedes editar. Permite aplicar transformaciones a las columnas para corregir características del texto, o transformar en columnas numéricas o de fecha.",
        );

    const dropd = contenedor.append("div").attr("class", "col-12");

    const framec = dropd
        .append("div")
        .attr("id", "gridBlock")
        .attr("class", "col-12 bloque");

    contenedor
        .append("div")
        .attr("id", "colBlock")
        .attr("class", "col-12 bloque");

    grid.mostrarGrid("#gridBlock");

    await herramienta_tabla("#colBlock");
    

}

async function promover() {
    const filas = d3.select("#tabla-cols").select("tbody").selectAll("tr");

    let instrc = [];
    filas.each(function(d, i) {
        const fila = d3.select(this);

        const accion = fila.select("select").node().value;
        const nuevo = fila.select("input").node().value;
        const nombre = d.cadena;

        instrc.push({nombre,nuevo,accion});

    });

    utils.setStatus("Procesando cambios...");
    utils.showSpinner();


    let respuesta;
    try {
        respuesta = await invoke("transformar",{transvec: instrc});
        console.log(respuesta);
    } catch(error) {
        utils.hideSpinner();
        utils.clearStatus();
        utils.showToast(`No se pudo hacer la transformación. Motivo: ${error}`,"danger");
        
        grid.mostrarGrid("#gridBlock");
        await herramienta_tabla("#colBlock");

        return
    } 

    window.appState.columnas = respuesta.columnas;
    window.appState.esquema = respuesta.esquema;

    grid.mostrarGrid("#gridBlock");
    await herramienta_tabla("#colBlock");
    
    utils.setStatus("Cambios realizados");
    utils.hideSpinner();
}

async function herramienta_tabla(selector) {
    const cols = d3.select(selector);
    cols.selectAll("*").remove();

    cols.append("h2").html("Validación de columnas");

    cols
        .append("p")
        .html(
        "Valida el nombre a usar en cada columna y el tipo de datos que debería de contener. La herramineta transformará los datos y ajustará algunos detalles para que la columna satisfaga los criterios",
        );

    let vcols;
    try {
        vcols = await invoke("validar_columnas",{columnas: window.appState.columnas});
    } catch(error) {
        utils.showToast(`No se pudo validar columnas. Motivo: ${error}`,"danger");
        return;
    }

    console.log(vcols);

    const nrow = cols.append("div").attr("class","row");
    nrow.append("div").attr("class","col-1");
    const ncols = nrow.append("div").attr("class","col-10");

    const tabla = ncols.append("table").attr("class","table").attr("id","tabla-cols");

    tabla.append("thead")
        .append("tr")
        .selectAll("th")
        .data(["Columnas","Tipos"])
        .join("th").html(d => d);

    const filas = tabla.append("tbody").selectAll("tr").data(vcols).join("tr");

    const td1s = filas.append("td");

    td1s.append("label").attr("for", (d, i) => "colinp_" + i).attr("class","form-label")
        .append("small")
        .html("nombre sugerido para: ")
        .append("span").append("strong").attr( "class",d => d.incidencia ? "Invalido" : "Valido")
        .html(d => d.cadena)

    td1s
        .append("input")
        .attr("type", "text")
        .attr("class", "form-control")
        .attr("id", (d, i) => "colinp_" + i)
        .attr("value", (d) => d.sugerido);

    const td2s = filas.append("td");
    
    td2s.append("label").attr("for", (d, i) => "colsel_" + i).attr("class","form-label")
        .append("small")
        .html(d => {
            const tipo = window.appState.esquema[d.cadena];
            return 'columna tipo <span class="' + tipo + '">' + tipo  + "</span> transformar a:"});

    const selecto = td2s.append("select")
        .attr("id", (d, i) => "colsel_" + i)
        .attr("class", "form-select");

    const tropc = [{label: "texto", value: "Texto"},
                   {label: "texto sin guiones", value: "TextoSinGuiones"},
                   {label: "texto en minúsculas", value: "TextoMinusculas"},
                   {label: "texto capitalizado", value: "TextoCapitalizado"},
                   {label: "numérica", value: "Numero"},
                   {label: "numérica coordenada", value: "Coordenada"},
                   {label: "fecha", value: "Fecha"},
                   {label: "anonimizar", value: "Anonimizar"},
                   {label: "eliminar columna", value: "EliminarColumna"}];

    selecto.selectAll("option")
        .data(tropc)
        .join("option")
        .attr("value", d => d.value)
        .html(d => d.label);
    
    selecto.each(function(d) {
        d3.select(this).selectAll("option")
        .property("selected", p => window.appState.esquema[d.cadena] === p.value);
    });

    cols
        .append("div")
        .append("button")
        .attr("class", "btn btn-primary")
        .html("Promover los cambios")
        .on("click", promover);
}