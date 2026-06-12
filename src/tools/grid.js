import { AllCommunityModule, ModuleRegistry, createGrid } from 'ag-grid-community';
ModuleRegistry.registerModules([AllCommunityModule]);
import * as d3 from 'd3';

const { invoke } = window.__TAURI__.core;

export function conectarGridInfinito(columnas, totalFilas, esquema) {
    const gridDiv = document.querySelector("#myGrid");
    gridDiv.innerHTML = "";

    const columnDefs = columnas.map(col => ({
        headerName: col,
        field: col,
        suppressMovable: true,
        headerClass: esquema[col],
        valueGetter: (params) => params.data?. [col] ?? '',
    }));

    const datasource = {
        getRows: async (params) => {
            try {
                const size = params.endRow - params.startRow;
                const filas = await invoke("obtener_bloque", {startRow: params.startRow, pageSize: size});

                params.successCallback(filas, totalFilas);
            } catch (error) {
                console.error("Error cargando bloque desde back:", error);
                params.failCallback();
            }
        }
    };

    const gridOptions = {
        columnDefs: columnDefs,
        rowModelType: 'infinite',
        cacheBlockSize: 100,
        maxBlocksInCache: 10,
        infiniteInitialRowCount: 1,

        defaultColDef: {
            flex: 1,
            minWidth: 150,
            resizable: true,
            sortable: false
        }
    };

    let grid = createGrid(gridDiv, gridOptions);
    grid.setGridOption('datasource', datasource);

    return grid;
}

export function mostrarGrid(selector) {

    console.log(window.appState);

    if (window.appState.grid) {
        try {
            window.appState.grid.destroy();
        } catch (e) {
          console.warn("Error al intentar destruir el grid anterior:", e);
        }
        window.appState.grid = null;
    }

    let reporte;
    if (window.appState.columnas && window.appState.total_filas && window.appState.esquema) {
        reporte = { ...window.appState };
    } else {
        console.log("salio")
        return;
    }

    const block = d3.select(selector);
    block.selectAll("*").remove();
    
    block.append("h2").html("Vista de los datos");

    const label = block.append("div").attr("class","label").append("p").html("Tipo de columna: ");
    label.selectAll(".laba").data(["Texto","Numero","Coordenada","Fecha"]).join("span").attr("class",d => d).html(d => d);

    block
        .append("div")
        .attr("id", "myGrid")
        .attr("class", "ag-theme-quartz")
        .style("height", "500px")
        .style("width", "100%");

    window.appState.grid = conectarGridInfinito(reporte.columnas,reporte.total_filas,reporte.esquema);
}