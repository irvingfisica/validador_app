const { invoke } = window.__TAURI__.core;
import { AllCommunityModule, ModuleRegistry, createGrid } from 'ag-grid-community';

import autoComplete from "@tarekraafat/autocomplete.js";
import * as d3 from 'd3';
import * as utils from "./utils.js";
import * as grid from "./grid.js";

export async function intface() {
    utils.enableTB("#comparaTool");

    const contenedor = d3.select("#mesaTrabajo");
    contenedor.selectAll("*").remove();

    const desc = contenedor.append("div").attr("class", "col-md-12");

    desc.append("h1").html("Herramienta de comparación");
    desc.append("p")
        .html(
        "Esta herramienta permite comparar el archivo que estas limpiando con una base de la PNDA.",
        );

    desc.append("p").html("Busca una institución")

    desc.append("div").attr("class","buscainst-wrapper mt-3")
        .append("input").attr("id","buscainst").attr("type","text")
        .attr("placeholder","Busca una institución...")
        .attr("autocomplete","off");

    desc.append("div").attr("class","mt-3").attr("id","recursos");

    let instituciones;
    try {
        utils.setStatus("Obteniendo instituciones...");
        utils.showSpinner();
        instituciones = await invoke("obtener_instituciones");
        console.log(instituciones);
        utils.hideSpinner();
        utils.clearStatus();
    } catch(error) {
        utils.hideSpinner();
        utils.clearStatus();
        utils.showToast(`No se pudo obtener las instituciones. Motivo: ${error}`,"danger");
        return;
    }
    

    const acjs = new autoComplete({
        selector: "#buscainst",
        placeHolder: "Busca una institución...",
        data: {
            src: instituciones,
            keys: ["display_name"]
        },
        diacritics: true,
        threshold: 1,
        debounce: 150,
        resultList: {
            maxResults: 10,
            noResults: true
        },
        resultItem: {
            highlight: true
        },
        events: {
            input: {
                selection(event) {
                    event.preventDefault();
                    const item = event.detail.selection.value;
                    acjs.input.value = item.display_name;
                    console.log(item);
                    obtenerConjuntos(item.name,item.display_name);
                },
            },
        },
    });

    async function obtenerConjuntos(name,institucion) {

        let conjuntos;
        try {
            utils.setStatus("Obteniendo recursos...");
            utils.showSpinner();
            conjuntos = await invoke("obtener_conjuntos",{institucion: name});
            utils.hideSpinner();
            utils.clearStatus();
        } catch(error) {
            utils.hideSpinner();
            utils.clearStatus();
            utils.showToast(`No se pudo obtener los conjuntos y recursos. Motivo: ${error}`,"danger");
            return;
        }
        

        const recursos = d3.select("#recursos");
        recursos.selectAll("*").remove();

        recursos.append("p").html("selecciona un conjunto y en su interior el recurso con el cual comparar la base de datos que estás limpiando")
        recursos.append("p").html("si tu base de datos es demasiado grande ten paciencia.")

        const items = recursos.append("div").attr("class","accordion")
            .attr("id","recursos_acc")
            .selectAll(".accordion-item").data(conjuntos)
            .join("div").attr("class","accordion-item");

        items.append("h2").attr("class","accordion-header")
            .append("button").attr("class","accordion-button collapsed")
            .attr("type","button")
            .attr("data-bs-toggle","collapse")
            .attr("data-bs-target",(d,i) => "#coll" + (i+1))
            .html(d => d.title);

        items.append("div").attr("id",(d,i) => "coll" + (i+1))
            .attr("class","accordion-collapse collapse")
            .attr("data-bs-parent","#recursos_acc")
            .append("div")
            .attr("class","accordion-body bg-body-tertiary")
            .append("table")
            .attr("class","table table-hover")
            .append("tbody")
            .selectAll("tr").data(d => d.resources.map(r => ({...r,conjunto: d.title})))
            .join("tr").append("td").html(p => p.name)
            .style("cursor","pointer")
            .on("click",async (e,p)  => {
                utils.setStatus("Descargando base desde PNDA...");
                utils.showSpinner();
                try {
                    window.refdata = await invoke("leer_referencia",{url:p.url});
                    window.refdata.recurso = p;
                    window.refdata.institucion = institucion;
                    console.log(refdata);
                    await comparar(window.refdata);
                } catch (error) {
                    utils.showToast(`No se pudo procesar la base. Motivo: ${error}`,"danger");
                } finally {
                    utils.hideSpinner();
                    utils.clearStatus();
                }
            });
            
    }

    const input = document.querySelector("#buscainst");

    input.addEventListener("focus", function () {
    this.value = "";
    });

    if (window.refdata) {
        await comparar(window.refdata);
    }

}

async function comparar(datos) {
    console.log(datos);
    const recursos = d3.select("#recursos");
    recursos.selectAll("*").remove();

    const info = recursos.append("div").attr("id","info");
    info.append("h2").html("Comparando con")
    info.append("p").attr("class","mb-1").html("<strong>Institución: </strong>" + datos.institucion);
    info.append("p").attr("class","mb-1").html("<strong>Conjunto: </strong>" + datos.recurso.conjunto);
    info.append("p").attr("class","mb-1").html("<strong>Recurso: </strong>" + datos.recurso.name);

    const mesa = recursos.append("div").attr("class","row mt-5 mb-5");

    const actual = mesa.append("div").attr("class","col-md-6")
    actual.append("div").attr("id","actual");
    const refere = mesa.append("div").attr("class","col-md-6")
    refere.append("div").attr("id","referencia");

    grid.mostrarGrid("#actual");
    grid.mostrarRef("#referencia",datos);

    const infoact = actual.append("div").attr("class","mt-3");
    actual.append("div").attr("class","mt-3").attr("id","statsact");

    infoact.append("p").attr("class","mb-1").html("<strong>Filas: </strong>" + window.appState.total_filas);
    infoact.append("p").attr("class","mb-1").html("<strong>Columnas: </strong>" + window.appState.columnas.length);
    infoact.append("p").attr("class","mb-1").selectAll("span").data(window.appState.columnas)
        .join("span").attr("class","me-2 badge text-bg-secondary")
        .style("cursor","pointer")
        .html(d => d)
        .on("click", async (e,p) => {
            let stats;
            if (window.appState.esquema[p] == "Numero") {
                try {
                    stats = await invoke("col_stats",{columna:p});
                    numericas("#statsact", stats);
                } catch (error) {
                    utils.showToast(`No se pudo mostrar la información. Motivo: ${error}`,"warning");
                }
            } else {
                try {
                    stats = await invoke("col_values",{columna:p});
                    categoricas("#statsact",stats,p);
                } catch (error) {
                    utils.showToast(`No se pudo mostrar la información. Motivo: ${error}`,"warning");
                }
            }
        });

    const refeact = refere.append("div").attr("class","mt-3");
    refere.append("div").attr("class","mt-3").attr("id","statsref");

    refeact.append("p").attr("class","mb-1").html("<strong>Filas: </strong>" + datos.total_filas);
    refeact.append("p").attr("class","mb-1").html("<strong>Columnas: </strong>" + datos.columnas.length);
    refeact.append("p").attr("class","mb-1").selectAll("span").data(datos.columnas)
        .join("span").attr("class","me-2 badge text-bg-secondary")
        .style("cursor","pointer")
        .html(d => d)
        .on("click", async (e,p) => {
            let stats;
            if (datos.esquema[p] == "Numero") {
                try {
                    stats = await invoke("col_stats_ref",{columna:p});
                    numericas("#statsref", stats);
                } catch (error) {
                    utils.showToast(`No se pudo mostrar la información. Motivo: ${error}`,"warning");
                }
            } else {
                try {
                    stats = await invoke("col_values_ref",{columna:p});
                    categoricas("#statsref",stats,p);
                } catch (error) {
                    utils.showToast(`No se pudo mostrar la información. Motivo: ${error}`,"warning");
                }
            }
        });
}

function numericas(selector,data) {
    const ancla = d3.select(selector);
    ancla.selectAll("*").remove();

    ancla.append("p").attr("class","mb-1").html("<strong>promedio: </strong>" + utils.formato(data.mean));
    ancla.append("p").attr("class","mb-1").html("<strong>varianza: </strong>" + utils.formato(data.var));
    ancla.append("p").attr("class","mb-1").html("<strong>mediana: </strong>" + utils.formato(data.median));
}

function categoricas(selector,data,col) {
    const ancla = d3.select(selector);
    ancla.selectAll("*").remove();

    const slice = data.slice(0,10);

    const tabla = ancla.append("table").attr("class","table");
    tabla.append("thead")
        .append("tr")
        .selectAll("th")
        .data(["Valor","Veces"])
        .join("th").html(d => d);

    const filas = tabla.append("tbody").selectAll("tr").data(slice).join("tr");
    filas.append("td").html(d => d[col]);
    filas.append("td").html(d => d["n"]);
}
