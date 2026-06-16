const { invoke } = window.__TAURI__.core;
import { AllCommunityModule, ModuleRegistry, createGrid } from 'ag-grid-community';


import * as d3 from 'd3';
import * as utils from "./utils.js";
import * as grid from "./grid.js";


export async function intface() {
    utils.enableTB("#categosTool");

    const contenedor = d3.select("#mesaTrabajo");
    contenedor.selectAll("*").remove();

    const desc = contenedor.append("div").attr("class", "col-md-12");

    desc.append("h1").html("Herramienta de valores");
    desc.append("p")
        .html(
        "La herramienta permite analizar las columnas textuales que codifican categorías y modificar sus valores para homologarlos.",
        );

    const dropd = contenedor.append("div").attr("class", "col-md-12");

    const framec = dropd
        .append("div")
        .attr("id", "gridBlock")
        .attr("class", "col-md-12 bloque");

    contenedor
        .append("div")
        .attr("id", "colBlock")
        .attr("class", "col-md-12 bloque");

    grid.mostrarGrid("#gridBlock");

    herramienta_columnas("#colBlock");

}

async function herramienta_columnas(selector) {
    const cols = d3.select(selector);
    cols.selectAll("*").remove();

    cols.append("h2").html("Editor de valores");

    const mtool = cols.append("div").attr("class","row");
    const tdiv = mtool.append("div").attr("class","col-md-6");
    const ediv = mtool.append("div").attr("class","col-md-6");

    ediv.append("p").html('Columna seleccionada: <span id="colactiva"></span>. Edita los valores incorrectos');
    ediv.append("div").attr("id","otherGrid")
            .attr("class", "ag-theme-quartz")
            .style("height", "400px")
            .style("width", "100%");

    ediv.append("div")
        .append("button")
        .attr("id","botprom")
        .attr("class", "btn btn-primary mt-3")
        .html("Promover los cambios");

    tdiv
        .append("p")
        .html(
        "Da click en la columna con la que deseas trabajar:",
        );

    const tabla = tdiv.append("table").attr("class","table table-hover")
                    .attr("id","tabla-cols");

    tabla.append("thead")
        .append("tr")
        .selectAll("th")
        .data(["Columna","Valores diferentes"])
        .join("th").html(d => d);

    let catcols = await Promise.all(
        Object.entries(window.appState.esquema)
                        .filter(ele => ele[1] == "Texto")
                        .map(async (ele) => {
                            try {
                                const categos = await invoke("col_categos", {columna: ele[0]});
                                return {
                                    nombre: ele[0],
                                    categos: categos
                                }

                            } catch (error) {
                                return {
                                    nombre: ele[0],
                                    categos: 0,
                                    error: error
                                }
                            }
                        })
                    );

    const limite_abs = 600;
    let max_unicos = Math.min(window.appState.total_filas*0.05,limite_abs);
    if (window.appState.total_filas < limite_abs) {
        max_unicos = window.appState.total_filas;
    }

    catcols = catcols
    .filter(ele => ele.categos <= max_unicos )
    .sort((a,b) => b.categos - a.categos);
    
    const filas = tabla.append("tbody").selectAll("tr").data(catcols).join("tr")
            .style("cursor","pointer")
            .on("click",function(event,d) {
                tabla.selectAll("tbody tr")
                    .classed("table-active", false);

                d3.select(this)
                    .classed("table-active", true);

                colcategos(event, d);
            });

    filas.append("td").html(d => d["nombre"]);
    filas.append("td").html(d => d["categos"]);

    tabla.select("tbody").select("tr").attr("class","table-active");
    colcategos(null,catcols[0]);
}

async function colcategos(e,d) {
    console.log(d);
    d3.select("#colactiva").html("<strong>" + d["nombre"] + "</strong>");
    const conteos = await invoke("col_values", {columna: d.nombre});

    const filas = conteos.map(ele => ({
        original: ele[d.nombre],
        nuevo: ele[d.nombre],
        n: ele.n
    }));

    if (window.otherGrid) {
        try {
            window.otherGrid.destroy();
        } catch (e) {
          console.warn("Error al intentar destruir el grid anterior:", e);
        }
        window.otherGrid = null;
    }

    const gridDiv = document.querySelector("#otherGrid");
    gridDiv.innerHTML = "";

    const columnDefs = [
    {
        field: "original",
        editable: false
    },
    {
        field: "nuevo",
        headerName: "Nuevo (editable)",
        editable: true,
        cellClassRules: {
        "celda-modificada": params =>
            params.data.original !== params.value
        }
    },
    {
        field: "n",
        editable: false,
        maxWidth: 80
    }
    ];

    const cambios = {};

    const gridOptions = {
        columnDefs: columnDefs,
        rowData: filas,
        autoSizeStrategy: {
            type: 'fitGridWidth',
            defaultMinWidth: 200,
        },
        onCellValueChanged: (params) => {
            if (params.colDef.field !== "nuevo") {
                return;
            }

            const original = params.data.original;
            const nuevo = params.newValue;

            if (original === nuevo) {
                delete cambios[original];
            } else {
                cambios[original] = nuevo;
            }

            params.api.refreshCells({
                rowNodes: [params.node],
                columns: ["nuevo"]
            });

            console.log(cambios);
        }
    };

    window.otherGrid = createGrid(gridDiv, gridOptions);

    d3.select("#botprom").on("click", async () => {

        utils.setStatus("Procesando cambios...");
        utils.showSpinner();

        await invoke("cambiar_valores", {
            columna: d.nombre,
            cambios: cambios
        });

        grid.mostrarGrid("#gridBlock");

        herramienta_columnas("#colBlock");

        utils.setStatus("Cambios realizados");
        utils.hideSpinner();

    });
}
