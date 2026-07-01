const { invoke } = window.__TAURI__.core;
import { AllCommunityModule, ModuleRegistry, createGrid } from 'ag-grid-community';
import { listen } from "@tauri-apps/api/event";

import * as d3 from 'd3';
import * as utils from "./utils.js";

export async function intface() {
    utils.limpiarEvento();
    utils.enableTB("#subirTool");

    window.colaUnlisten = await listen(
        "cola-actualizada",
        async () => {
            await actualizarCola();
        }
    );
    
    const contenedor = d3.select("#mesaTrabajo");
    contenedor.selectAll("*").remove();

    const desc = contenedor.append("div").attr("class", "col-md-12");

    desc.append("h1").html("Herramienta de subida");
    desc.append("p")
        .html(
        "Esta herramienta permite subir archivos al repositorio de la PNDA. Solo está destinada a usarse por el personal de la DDA. Es necesario que tu VPN esté conectado.",
        );

    let sugerido = await invoke("ruta_sugerida");

    const controles = desc.append("div").attr("class","row");
    controles.append("h3").html("Introduce los siguientes datos");

    const filepar = controles.append("div").attr("class","col-md-12");
    filepar.append("label").attr("for","filename").attr("class","form-label mt-3").html("Nombre de archivo a usar:");
    filepar.append("input").attr("type","text").attr("class","form-control").attr("id","filename").attr("value",sugerido);

    const conectpar = controles.append("div").attr("class","col-md-6");
    const folderpar = controles.append("div").attr("class","col-md-6");

    conectpar.append("label").attr("for","usuario").attr("class","form-label mt-3").html("Usuario:");
    conectpar.append("input").attr("type","text").attr("class","form-control").attr("id","usuario");

    folderpar.append("label").attr("for","server").attr("class","form-label  mt-3").html("Servidor:");
    folderpar.append("input").attr("type","text").attr("class","form-control").attr("id","server");

    
    conectpar.append("label").attr("for","institucion").attr("class","form-label  mt-3").html("Institución:");
    conectpar.append("input").attr("type","text").attr("class","form-control").attr("id","institucion");

    folderpar.append("label").attr("for","conjunto").attr("class","form-label  mt-3").html("Conjunto:");
    folderpar.append("input").attr("type","text").attr("class","form-control").attr("id","conjunto");

    controles.append("div")
        .append("button")
        .attr("id","botprom")
        .attr("class", "btn btn-primary mt-4 mb-5")
        .html("Subir archivo");

    if (window.appState.server) {
        d3.select("#server").node().value = window.appState.server
    };

    if (window.appState.user) {
        d3.select("#usuario").node().value = window.appState.user
    };

    d3.select("#botprom").on("click", async () => {
        const servidor = window.appState.server = d3.select("#server").node().value.trim();
        const usuario = window.appState.user = d3.select("#usuario").node().value.trim();

        const institucion = d3.select("#institucion").node().value.trim();
        const conjunto = d3.select("#conjunto").node().value.trim();

        const nombre = d3.select("#filename").node().value.trim();

        const campos = {
            usuario,
            servidor,
            institucion,
            conjunto,
            nombre
        };

        for (const [llave, valor] of Object.entries(campos)) {
            if (!valor) {
                utils.showToast(`El campo "${llave}" es obligatorio.`,"danger");
                return
            }

            if (/\s/.test(valor)) {
                utils.showToast(`El campo "${llave}" no puede contener espacios.`,"danger");
                return;
            } 
        }

        console.log(window.appState.server, window.appState.user, institucion, conjunto);

        const config = {
            host: servidor, 
            usuario: usuario, 
            institucion: institucion, 
            conjunto: conjunto,
            archivo: nombre
        }

        try {

            utils.setStatus("Agregando datos a cola de subida...");
            utils.showSpinner();

            let id = await invoke("agregar_subida", {config: config});
            utils.showToast(`Archivo agregado a la cola de subida (ID ${id})`,"success");

            let cola = await invoke("obtener_cola");

            console.log(cola);
        } catch(error) {
            utils.hideSpinner();
            utils.showToast(`No se pudo agregar el archivo. Motivo: ${error}`,"danger");
        } finally {
            utils.clearStatus();
            utils.hideSpinner();
        }

        actualizarCola();
    });

    const cola = contenedor.append("div").attr("class","row mb-5");
    cola.append("h3").html("Cola de subidas");

    cola.append("div").attr("id","tablaCola");

    actualizarCola();

}



async function actualizarCola() {
    if (!document.querySelector("#tablaCola")) {
        return;
    }

    const cola = await invoke("obtener_cola");
    cola.reverse();
    console.log(cola);

    const tabla = d3.select("#tablaCola");

    tabla.selectAll("*").remove();

    const filas = tabla.selectAll(".fila").data(cola).enter().append("div").attr("class","fila");

    filas.append("hr");

    filas.append("p").html(d => `<strong>Archivo:</strong> ${d.archivo_remoto}`);
    filas.append("p").html(d => "Intitución: " + d.institucion + ", Conjunto: " + d.conjunto);
    filas.append("p").append("strong").html(d => {if(d.resultado?.url) {return d.resultado.url}});
    filas.append("p")
    .html(d => {
        let tcolor = "text-warning";
        if (d.estado == "Completado") {
            tcolor = "text-success"
        }

        if (d.estado == "Error") {
            tcolor = "text-danger"
        }
        return `<strong>Estado: <span class="${tcolor}">${d.estado}</span></strong>`
    });

    filas.each(function(d) {
        if (d.estado === "Error") {
            d3.select(this).append("p").style("color","red").html(d.error);

            d3.select(this)
            .append("button")
            .attr("class", "btn btn-warning btn-sm")
            .text("Reintentar")
            .on("click", async () => {

                try {
                    await invoke(
                        "reintentar_subida",
                        { id: d.id }
                    );

                } catch (e) {
                    utils.showToast(
                        `No se pudo reintentar: ${e}`,
                        "warning"
                    );
                } finally {
                    await actualizarCola()
                }
            });
        }
    })
}

