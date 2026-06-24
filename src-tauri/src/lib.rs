use chardetng::{EncodingDetector,Iso2022JpDetection, Utf8Detection};
use polars::prelude::*;
use tokio::io::AsyncWriteExt;
use tokio::sync::Notify;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::io::{Read,BufReader,Cursor};
use std::path::{Path,PathBuf};
use std::fs::File;
use std::io::Write;
use std::env;
use serde::{Serialize,Deserialize};
use std::sync::Mutex;
use tauri::State;
use tauri::Manager;
use serde_json::Value;
use regex::Regex;
use std::sync::LazyLock;
use unicode_normalization::UnicodeNormalization;
use std::sync::OnceLock;
use sha2::{Digest, Sha256};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, ACCEPT_LANGUAGE, CACHE_CONTROL, PRAGMA, USER_AGENT};
use futures::stream::{self, StreamExt};
use std::time::Duration;
use russh_sftp::protocol::OpenFlags;
use russh_sftp::client::SftpSession;
use tauri::Emitter;

fn cliente_ckan() -> Result<reqwest::Client, String> {
    let mut headers = HeaderMap::new();

    headers.insert(USER_AGENT, HeaderValue::from_static(
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) \
             AppleWebKit/537.36 (KHTML, like Gecko) \
             Chrome/122.0.0.0 Safari/537.36"));

    headers.insert(
        ACCEPT,
        HeaderValue::from_static(
            "application/json, text/plain, */*"
        ),
    );

    headers.insert(
        ACCEPT_LANGUAGE,
        HeaderValue::from_static(
            "es-MX,es;q=0.9,en;q=0.8"
        ),
    );

    headers.insert(
        CACHE_CONTROL,
        HeaderValue::from_static("no-cache"),
    );

    headers.insert(
        PRAGMA,
        HeaderValue::from_static("no-cache"),
    );

    reqwest::Client::builder().default_headers(headers)
        .gzip(true)
        .brotli(true)
        .deflate(true)
        .tcp_keepalive(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(60))
        .build().map_err(|e| e.to_string())
}

#[derive(Debug, Deserialize)]
struct PackageSearchResponse {
    count: usize,
    results: Vec<Conjunto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conjunto {
    pub id: String,
    pub name: String,
    pub title: String,
    #[serde(default)]
    pub institucion: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Institucion {
    pub id: String,
    pub name: String,
    pub display_name: String
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CacheInstituciones {
    pub instituciones: Vec<Institucion>
}

#[derive(Deserialize)]
struct CkanResponse<T> {
    success: bool,
    result: T
}

#[derive(Deserialize)]
pub struct ConexionConfig {
    usuario: String,
    host: String,
    institucion: String,
    conjunto: String,
    archivo: String
}

pub struct RepoConexion {
    ssh: russh::client::Handle<ClientHandler>,
    sftp: SftpSession
}

#[derive(Clone, Serialize)]
pub struct SubidaRepo {
    size_original: u64,
    size_subido: u64,
    url: String
}

#[derive(Clone, Serialize)]
pub enum EstadoSubida {
    EnCola,
    Subiendo,
    Completado,
    Error
}

#[derive(Clone, Serialize)]
pub struct TrabajoSubida {
    pub id: u64,

    pub usuario: String,
    host: String,

    pub institucion: String,
    pub conjunto: String,

    pub archivo_local: String,
    pub archivo_remoto: String,

    pub estado: EstadoSubida,

    pub resultado: Option<SubidaRepo>,
    pub error: Option<String>
}

pub struct ColaSubidas {
    pub pendientes: Mutex<VecDeque<TrabajoSubida>>,
    pub en_proceso: Mutex<Option<TrabajoSubida>>,
    pub historico: Mutex<Vec<TrabajoSubida>>,
    pub notify: Notify
}

pub struct ContenedorDatos {
    pub dataframe: Mutex<Option<DataFrame>>,
    pub ruta_original: Mutex<Option<String>>,
    pub ruta_sugerida: Mutex<Option<String>>
}

#[derive(Serialize)]
pub struct CaracterCorrupto {
    pub caracter: String,
    pub filas: Vec<u64>
}

#[derive(Serialize)]
pub struct ValidacionCadena {
    cadena: String,
    sugerido: String,
    incidencia: bool
}

#[derive(Serialize)]
pub struct ReporteCsv {
    pub encoding_detectado: String,
    pub requiere_conversion: bool,
    pub caracteres_corruptos: Vec<CaracterCorrupto>,
    pub total_filas: usize,
    pub columnas: Vec<String>,
    pub esquema: BTreeMap<String, TipoColumna>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ResultadoTransformacion {
    Exito,
    Error(String)
}

#[derive(Serialize)]
pub struct ReporteTransformacion {
    pub columnas: Vec<String>,
    pub esquema: BTreeMap<String, TipoColumna>,
    pub resultados: HashMap<String, ResultadoTransformacion>
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum TipoColumna {
    Numero,
    Coordenada,
    Fecha,
    Texto,
}

impl TipoColumna {
    pub fn to_polartype(&self) -> DataType {
        match self {
            TipoColumna::Numero => DataType::Float64,
            TipoColumna::Fecha => DataType::Date,
            TipoColumna::Texto => DataType::String,
            TipoColumna::Coordenada => DataType::Float64
        }
    }

    pub fn from_polartype(dt: &DataType, coord: bool) -> Self {
        match dt {
            DataType::Float64 => {
                if coord {TipoColumna::Coordenada } else { TipoColumna::Numero}}
            DataType::Date => TipoColumna::Fecha,
            _ => TipoColumna::Texto
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum OpcionesTrans {
    Texto,
    TextoSinGuiones,
    TextoMinusculas,
    TextoCapitalizado,
    Numero,
    Coordenada,
    Fecha,
    Anonimizar,
    EliminarColumna
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Transformacion {
    nombre: String,
    nuevo: String,
    accion: OpcionesTrans,
}

#[tauri::command]
async fn exportar_csv(
    ruta: String,
    state: State<'_, ContenedorDatos>
) -> Result<(), String> {

    let guardado = state.dataframe
        .lock()
        .map_err(|_| "Error al bloquear el estado")?;

    let df = guardado
        .as_ref()
        .ok_or("No hay dataframe cargado")?;

    escribir_csv(df, &ruta).map_err(|e| format!("Error al escribir CSV: {}", e))
}

#[tauri::command]
async fn ruta_sugerida(
    state: State<'_, ContenedorDatos>
) -> Result<String, String> {

    let ruta = state.ruta_sugerida
        .lock()
        .map_err(|_| "Error al bloquear estado")?;

    ruta.clone().ok_or("No hay archivo cargado".to_string())
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
async fn leer_csv(ruta: String, state: State<'_, ContenedorDatos>) -> Result<ReporteCsv, String> {
    let path = Path::new(&ruta);
    if !path.exists() {
        return Err("El archivo no existe.".to_string());
    }

    let file = File::open(&ruta).map_err(|e| e.to_string())?;
    let mut reader = BufReader::new(file);
    let mut buffer_inicio = vec![0; 4096];
    let bytes_leidos =  reader.read(&mut buffer_inicio).map_err(|e| e.to_string())?;

    let mut file_completo = File::open(&ruta).map_err(|e| e.to_string())?;
    let mut bytes_puros = Vec::new();
    file_completo.read_to_end(&mut bytes_puros).map_err(|e| e.to_string())?;

    let mut mapa_caracteres: BTreeMap<char, BTreeSet<u64>> = BTreeMap::new();
    let mut nombre_encoding = "UTF-8".to_string();
    let mut requiere_conversion = false;

    let (texto_convertido, _encod, tuviera_errores) = encoding_rs::UTF_8.decode(&bytes_puros);

    let contenido_final = if tuviera_errores {
        let mut detector = EncodingDetector::new(Iso2022JpDetection::Deny);
        detector.feed(&buffer_inicio[..bytes_leidos], true);
        let encoding_alternativo = detector.guess(None, Utf8Detection::Allow);

        nombre_encoding = encoding_alternativo.name().to_string();
        requiere_conversion = true;

        let (texto_alt, _encalt, _err) = encoding_alternativo.decode(&bytes_puros);

        for (indice, linea) in texto_alt.lines().enumerate() {
            let fila_actual = (indice + 1) as u64;
            for c in linea.chars() {
                if es_caracter_corrupto(c) {
                    mapa_caracteres.entry(c).or_default().insert(fila_actual);
                }
            }
        }

        texto_alt.into_owned()
    } else {
        for (indice, linea) in texto_convertido.lines().enumerate() {
            let fila_actual = (indice + 1) as u64;
            for c in linea.chars() {
                if es_caracter_corrupto(c) {
                    mapa_caracteres.entry(c).or_default().insert(fila_actual);
                }
            }
        }

        texto_convertido.into_owned()
    };

    let caracteres_corruptos: Vec<CaracterCorrupto> = mapa_caracteres.into_iter().map(|(caracter,filas)| CaracterCorrupto {
        caracter: caracter.to_string(),
        filas: filas.into_iter().collect()
    }).collect();

    let mut header_cursor = Cursor::new(contenido_final.as_bytes());
    let extractor_headers = CsvReader::new(&mut header_cursor).with_options(
            CsvReadOptions::default()
            .with_has_header(true)
            .with_n_rows(Some(0))
            .with_infer_schema_length(Some(0))
        );

    let df_headers = extractor_headers.finish().map_err(|e| format!("Error al mapear columnas: {}", e))?;
    let ncols = df_headers.width();
    let types = vec![DataType::String;ncols];

    let cursor = Cursor::new(contenido_final);

    let mut df = CsvReader::new(cursor).with_options(
            CsvReadOptions::default()
            .with_has_header(true)
            .with_dtype_overwrite(Some(Arc::new(types)))
        ).finish().map_err(|e| format!("No se pudo procesar el texto: {}", e))?;

        df = df.lazy()
        .with_columns([
            col("*").str().strip_chars(lit(" "))
        ]).collect().map_err(|e| format!("Error limpiando espacios extra: {}", e))?;

        df = df.lazy().filter(any_horizontal([col("*").is_not_null().and(col("*").neq(lit("")))]).map_err(|_|"Error en el filtrado de filas vacías")?)
        .collect().map_err(|e| format!("Error limpiando filas vacías: {}", e))?;

        df = castear_frame(df, TipoColumna::Fecha);
        df = castear_frame(df, TipoColumna::Numero);

        let mut esquema = BTreeMap::new();
        for col in df.columns() {
            let nombre_columna = col.name().to_string();
            let tipo_final = TipoColumna::from_polartype(col.dtype(), false);
            esquema.insert(nombre_columna, tipo_final);
        }

    let columnas: Vec<String> = df.get_column_names().iter().map(|s| s.to_string()).collect();
    let total_filas = df.height();

    //validar_columnas(columnas.clone()).await?;

    let mut guardado = state.dataframe.lock().map_err(|_| "Error al bloquear el estado")?;
    *guardado = Some(df);

    let mut ruta_guardada = state.ruta_original
    .lock()
    .map_err(|_| "Error al bloquear estado")?;

    *ruta_guardada = Some(ruta.clone());

    let mut ruta_sugerida = state.ruta_sugerida
    .lock()
    .map_err(|_| "Error al bloquear estado")?;

    *ruta_sugerida = Some(nombre_archivo_sugerido(ruta.as_str()));

    Ok(ReporteCsv { encoding_detectado: nombre_encoding, requiere_conversion, caracteres_corruptos, total_filas, columnas, esquema })
}

#[tauri::command]
async fn transformar(transvec: Vec<Transformacion>,state: State<'_, ContenedorDatos>) -> Result<ReporteTransformacion,String> {
    let mut guardado = state.dataframe.lock().map_err(|_| "Error al bloquear el estado")?;
    let df = guardado.as_mut().ok_or("No hay dataframe")?;

    let mut columnas_eliminar = Vec::new();
    let mut coords = HashSet::new();
    let mut resultados = HashMap::new();

    for t in transvec {

        if matches!(t.accion, OpcionesTrans::EliminarColumna) {
            columnas_eliminar.push(t.nombre.clone());
            continue;
        }

        let serie = df.column(&t.nombre).map_err(|e|format!("No se pudo obtener columna: {}", e))?;

        match aplicar_transformacion(serie.as_materialized_series(), t.accion) {
            Ok(nueva) => {
                df.replace(&t.nombre, nueva.into_column()).map_err(|_| "No se pudo remplazar serie")?;
                df.rename(&t.nombre,t.nuevo.clone().into()).map_err(|e| format!("No se pudo cambiar nombre: {}", e))?;


                if matches!(t.accion, OpcionesTrans::Coordenada) {
                    coords.insert(t.nuevo.clone());
                }

                resultados.insert(t.nuevo.clone(),ResultadoTransformacion::Exito);
            },

            Err(e) => {
                df.rename(&t.nombre,t.nuevo.clone().into()).map_err(|e| format!("No se pudo cambiar nombre: {}", e))?;

                resultados.insert(t.nuevo.clone(),ResultadoTransformacion::Error(e.to_string()));
            }
        }
    }

    for col in columnas_eliminar {
        df.drop_in_place(&col).map_err(|e| format!("No se pudo eliminar columna: {}",e))?;
    }

    let columnas: Vec<String> = df.get_column_names().iter().map(|s| s.to_string()).collect();

    let mut esquema = BTreeMap::new();
        for col in df.columns() {
            let nombre_columna = col.name().to_string();
            let es_coordenada = coords.contains(col.name().as_str());
            let tipo_final = TipoColumna::from_polartype(col.dtype(), es_coordenada);
            esquema.insert(nombre_columna, tipo_final);
    };

    Ok(ReporteTransformacion {
        columnas,
        esquema,
        resultados
    })
}

fn aplicar_transformacion(serie: &Series, accion: OpcionesTrans) -> Result<Series, PolarsError> {
    match accion {
        OpcionesTrans::Texto => transformar_texto(serie),
        OpcionesTrans::TextoSinGuiones => texto_sin_guiones(serie),
        OpcionesTrans::TextoMinusculas => texto_minusculas(serie),
        OpcionesTrans::TextoCapitalizado => texto_capitalizado(serie),
        OpcionesTrans::Numero => transformar_a_numerica(serie),
        OpcionesTrans::Coordenada => transformar_a_coordenada(serie),
        OpcionesTrans::Fecha => transformar_a_fecha(serie),
        OpcionesTrans::Anonimizar => anonimizar(serie),
        OpcionesTrans::EliminarColumna => unreachable!(),
    }
}

#[tauri::command]
async fn obtener_bloque(start_row: i64, page_size: i64,state: State<'_, ContenedorDatos>) -> Result<Value, String> {
    let guardado = state.dataframe.lock().map_err(|_| "Error al bloquear el estado")?;

    let df = guardado.as_ref().ok_or("No hay dataframe")?;

    let df_bloque = df.clone().slice(start_row, page_size as usize);

    let mut buf = Vec::new();
    JsonWriter::new(&mut buf).with_json_format(JsonFormat::Json).finish(&mut df_bloque.clone()).map_err(|e| format!("Error de formato al escribir JSON: {}", e))?;

    let rows: Value = serde_json::from_slice(&buf).map_err(|e| format!("Error al estructurar el JSON: {}", e))?;

    Ok(rows)
}

#[tauri::command]
async fn validar_columnas(columnas: Vec<String>) -> Result<Value, String> {
/*     if columnas.iter().any(|x| x.contains("_duplicate_") || x == "") {
        return Err("Hay columnas vacías o duplicadas".to_string())
    }; */

    let mut validacion: Vec<ValidacionCadena> = columnas.iter().map(|col| validar_cadena(col)).collect();

    let mut usados = HashSet::new();

    for v in &mut validacion {
        let base = v.sugerido.clone();

        if usados.insert(base.clone()) {
            continue;
        }

        let mut n = 2;

        loop {
            let candidato = format!("{}_0{}", base, n);

            if usados.insert(candidato.clone()) {
                v.sugerido = candidato;
                break;
            }

            n += 1;
        }
    }

    let json_val = serde_json::to_value(validacion).map_err(|e|e.to_string())?;

    Ok(json_val)
}

#[tauri::command]
async fn col_categos(columna: String, state: State<'_, ContenedorDatos>) -> Result<usize,String> {
    let mut guardado = state.dataframe.lock().map_err(|_| "Error al bloquear el estado")?;
    let df = guardado.as_mut().ok_or("No hay dataframe")?;

    let serie = df.column(&columna).map_err(|e|format!("No se pudo obtener columna: {}", e))?;

    serie.n_unique().map_err(|e|format!("No se pudo obtener el número de categorías: {}", e))
}

#[tauri::command]
async fn col_values(columna: String, state: State<'_, ContenedorDatos>) -> Result<Value,String> {
    let mut guardado = state.dataframe.lock().map_err(|_| "Error al bloquear el estado")?;
    let df = guardado.as_mut().ok_or("No hay dataframe")?;

    let serie = df.column(&columna).map_err(|e|format!("No se pudo obtener columna: {}", e))?;
    let s = serie.as_materialized_series();

    let df_bloque = s.value_counts(true, true, "n".into(), false).map_err(|e| format!("Error al obtener conteos: {}", e))?;

    let mut buf = Vec::new();
    JsonWriter::new(&mut buf).with_json_format(JsonFormat::Json).finish(&mut df_bloque.clone()).map_err(|e| format!("Error de formato al escribir JSON: {}", e))?;

    let rows: Value = serde_json::from_slice(&buf).map_err(|e| format!("Error al estructurar el JSON: {}", e))?;

    Ok(rows)
}

#[tauri::command]
async fn cambiar_valores(columna: String,cambios: HashMap<String,String>,state: State<'_, ContenedorDatos>) -> Result<(), String> {
    let mut guardado = state.dataframe.lock().map_err(|_| "Error al bloquear el estado")?;
    let df = guardado.as_mut().ok_or("No hay dataframe")?;

    if cambios.is_empty() {
        return Ok(());
    }

    // Construir expresión encadenada de reemplazos
    let mut expr = col(&columna);

    for (original, nuevo) in &cambios {
        expr = when(col(&columna).eq(lit(original.as_str())))
            .then(lit(nuevo.as_str()))
            .otherwise(expr);
    }

    *df = df.clone().lazy()
        .with_column(expr.alias(&columna))
        .collect()
        .map_err(|e| format!("Error al cambiar valores: {}", e))?;

    Ok(())

}

#[tauri::command]
async fn obtener_conjuntos(institucion: String) -> Result<Vec<Conjunto>, String> {
    let cliente = cliente_ckan()?;
    let conjuntos = obtener_conjuntos_institucion(&institucion, &cliente).await?;

    Ok(conjuntos)
}

#[tauri::command]
async fn obtener_instituciones() -> Result<Vec<Institucion>, String> {
    let cliente = cliente_ckan()?;
    let lista_actual = obtener_lista_instituciones(&cliente).await?;

    let mut cache = cargar_cache();

    let mapa: HashMap<String, Institucion> = cache.instituciones.iter().cloned().map(|x| (x.name.clone(), x)).collect();

    let faltantes: Vec<String> = lista_actual.iter().filter(|nombre| !mapa.contains_key(*nombre)).cloned().collect();

    let nuevas: Vec<Institucion> = stream::iter(faltantes).map(|nombre| {
        let cliente = cliente.clone();
        async move {
        obtener_detalle_institucion(&nombre, &cliente).await}
    }).buffer_unordered(10).filter_map(|x| async move {
        match x {
            Ok(i) => Some(i),
            Err(e) => {
                eprintln!("Error: {}", e);
                None
            }
        }
    }).collect().await;

    for institucion  in nuevas {
            cache.instituciones.push(institucion);
    }

    cache.instituciones.retain(|i| {lista_actual.contains(&i.name)});

    guardar_cache(&cache)?;

    let mapa_final: HashMap<String, Institucion> = cache.instituciones.iter().cloned().map(|x| (x.name.clone(), x)).collect();

    let mut resultado = Vec::new();

    for nombre in lista_actual {
        if let Some(i) = mapa_final.get(&nombre) {
            resultado.push(i.clone());
        }
    }

    resultado.sort_by(|a,b| {
        a.display_name.cmp(&b.display_name)
    });

    Ok(resultado)
}

#[tauri::command]
async fn subir_repo(config: ConexionConfig, state: State<'_, ContenedorDatos>) -> Result<SubidaRepo, String> {
    let df = {
        let guardado = state
            .dataframe
            .lock()
            .map_err(|_| "Error al bloquear el estado")?;

        guardado
            .as_ref()
            .ok_or("No hay dataframe")?
            .clone()
    };

    let ruta_sugerida = config.archivo;

    let ruta_temporal = exportar_temporal(df, &ruta_sugerida).map_err(|e|format!("No se pudo guardar el archivo temporal {e}"))?;

    let repo = conectar_repo(&config.host, &config.usuario).await.map_err(|e|format!("No se pudo conectar al repo {e}"))?;
    let folder_remoto = crear_ruta_remota(&repo, &config.institucion, &config.conjunto).await?;
    let ruta_remota = folder_remoto + &ruta_sugerida;

    let subido = subir_archivo_repo(&repo, &ruta_temporal, &ruta_remota).await?;

    destruir(repo).await;

    Ok(subido)
}

#[tauri::command]
async fn agregar_subida(
    config: ConexionConfig, state: State<'_, ContenedorDatos>, cola: State<'_, Arc<ColaSubidas>>) -> Result<u64,String> {

    let df = {
    let guardado = state
        .dataframe
        .lock()
        .map_err(|_| "Error al bloquear el estado")?;

    guardado
        .as_ref()
        .ok_or("No hay dataframe")?
        .clone()
    };

    let ruta_sugerida = config.archivo;

    let ruta_temporal = exportar_temporal(df, &ruta_sugerida).map_err(|e|format!("No se pudo guardar el archivo temporal {e}"))?;

    let id =
        std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    let trabajo = TrabajoSubida {
        id,
        usuario: config.usuario.clone(),
        host: config.host.clone(),
        institucion: config.institucion.clone(),
        conjunto: config.conjunto.clone(),
        archivo_local: ruta_temporal.to_string_lossy().to_string(),
        archivo_remoto: ruta_sugerida,
        estado: EstadoSubida::EnCola,
        resultado: None,
        error:None
    };

    cola.pendientes.lock().unwrap().push_back(trabajo);

    cola.notify.notify_one();

    Ok(id)
}

#[tauri::command]
fn obtener_cola(cola: State<'_, Arc<ColaSubidas>>) -> Vec<TrabajoSubida> {
    let mut resultado = Vec::new();

    resultado.extend(cola.historico.lock().unwrap().clone());

    if let Some(actual) = cola.en_proceso.lock().unwrap().clone()
    {
        resultado.push(actual);
    }

    resultado.extend(cola.pendientes.lock().unwrap().iter().cloned());

    resultado
}

#[tauri::command]
async fn reintentar_subida(id: u64, cola:State<'_, Arc<ColaSubidas>>) -> Result<(), String> {

    let mut trabajo = {
        let mut historico = cola.historico.lock().map_err(|_|"Error al bloquear historico")?;

        let pos = historico
            .iter()
            .position(|t| t.id == id)
            .ok_or("Trabajo no encontrado")?;

        historico.remove(pos)
    };

    if !matches!(trabajo.estado, EstadoSubida::Error) {
        return Err("El trasbajo no tiene error".into());
    }

    if !Path::new(&trabajo.archivo_local).exists() {
        return Err("El archivo temporal ya no existe".into());
    }

    trabajo.estado = EstadoSubida::EnCola;
    trabajo.error = None;

    cola.pendientes.lock().map_err(|_|"Error al bloquear cola")?.push_back(trabajo);

    cola.notify.notify_one();

    Ok(())
}


#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(ContenedorDatos { 
            dataframe: Mutex::new(None),
            ruta_original: Mutex::new(None),
            ruta_sugerida: Mutex::new(None),
        })
        .manage(Arc::new(ColaSubidas { 
                pendientes: Mutex::new(VecDeque::new()),
                en_proceso: Mutex::new(None),
                historico: Mutex::new(Vec::new()),
                notify: Notify::new()
            }))
        .invoke_handler(tauri::generate_handler![
            leer_csv, 
            obtener_bloque, 
            validar_columnas, 
            transformar, 
            col_categos, 
            col_values, 
            cambiar_valores, 
            exportar_csv, 
            ruta_sugerida, 
            obtener_instituciones,
            obtener_conjuntos,
            subir_repo,
            agregar_subida,
            obtener_cola,
            reintentar_subida
            ])
            .setup(|app| {
                let cola = app.state::<Arc<ColaSubidas>>().inner().clone();

                if let Err(e) = limpiar_temporales() {
                    eprintln!("Error limpiando temporales: {e}");
                }

                let app_handle = app.handle().clone();

                tauri::async_runtime::spawn(async move {
                    worker_subidas(cola, app_handle).await;
                });

                Ok(())
            })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn es_caracter_corrupto(c: char) -> bool {
    let code = c as u32;

    // 1. Validar el rombo de reemplazo directamente
    if c == '\u{FFFD}' {
        return true;
    }
    // 2. Control chars (excluyendo tab, LF, CR)
    if code < 32 && code != 9 && code != 10 && code != 13 {
        return true;
    }
    // 3. Delete char
    if code == 127 {
        return true;
    }

    // 4. Expresiones regulares NORMAL y TYPICAL para español/datos comunes
    // Caracteres NORMALES: a-z, A-Z, 0-9, espacios y puntuación básica
    let es_normal = c.is_ascii_alphanumeric() || c.is_ascii_whitespace() || 
                    ".,;:()\"'¿?¡!-_/".contains(c);

    if !es_normal {
        // Si no es normal, revisamos si al menos es de los TÍPICOS aceptados en español (acentos, eñes, símbolos de pesos, etc.)
        let es_tipico = "áéíóúÁÉÍÓÚñÑüÜ“”\"%°ºª€$".contains(c);
        if es_tipico {
            return false; // Es un acento o eñe perfectamente válido
        } else {
            return true; // Es un "badChar" real (un Mojibake o símbolo extraño)
        }
    }

    false
}

fn castear_columna(df: DataFrame, columna: &str, tipo: TipoColumna) -> DataFrame {

    match tipo {
        TipoColumna::Fecha => {
            for formato in ["%Y-%m-%d", "%d-%m-%Y","%Y/%m/%d", "%d/%m/%Y"] {
                let expr = col(columna).str().to_date(StrptimeOptions {
                format: Some(formato.into()),
                strict: true,
                exact: true,
                cache: true,
            });
                match df.clone().lazy().with_column(expr).collect() {
                    Ok(df_transformado) => {
                        let nulos = df_transformado.column(columna)
                            .map(|c| c.null_count()).unwrap_or(0);
                        let total = df_transformado.height();
                        if nulos < total {
                            return df_transformado;
                        }
                        continue;
                    },
                    Err(_) => continue
                }
            };

            return df;
        },
        TipoColumna::Numero => {
            if df.column(columna).map(|c| c.dtype() == &DataType::Date).unwrap_or(false) {
                return df;
            } else {
                let expr = col(columna).strict_cast(tipo.to_polartype());
                match df.clone().lazy().with_column(expr).collect() {
                    Ok(df_transformado) => return df_transformado,
                    Err(_) => return df
                }
            }
        },
        _ => {
            let expr = col(columna).strict_cast(tipo.to_polartype());
                match df.clone().lazy().with_column(expr).collect() {
                    Ok(df_transformado) => return df_transformado,
                    Err(_) => return df
                }
        }
    }

}

fn castear_frame(df: DataFrame, tipo: TipoColumna) -> DataFrame {
    let columns: Vec<String> = df.get_column_names().iter().map(|name| name.to_string()).collect();

    let mut temporal = df;

    for nombre in columns {
        temporal = castear_columna(temporal, &nombre, tipo);
    }

    temporal
}

fn validar_cadena(cadena: &str) -> ValidacionCadena {
    static RE_NO_ALFA: OnceLock<Regex> = OnceLock::new();
    let re_no_alfa = RE_NO_ALFA.get_or_init(|| Regex::new(r"[^a-z0-9]").unwrap());

    let mut limpia = cadena.trim().to_lowercase();
    limpia = limpia.replace('ñ', "ni");

    limpia = limpia.nfd().filter(|c|!('\u{0300}'..='\u{036f}').contains(c)).collect::<String>();

    limpia = re_no_alfa.replace_all(&limpia, "_").into_owned();

    let prohibidas = ["el","la","los","las","un","una","unos","unas","a","que",
                                "ante","bajo","cabe","con","contra","de","del","durante",
                                "en","entre","mediante","para","segun","por",
                                "sin","so","sobre","tras","versus","y","o","e","u"];

    let palabras: Vec<&str> = limpia
        .split('_')
        .filter(|p| !p.is_empty() && !prohibidas.contains(p))
        .collect();

    limpia = palabras.join("_");

    let empieza_con_numero = limpia.chars().next().is_some_and(|c| c.is_ascii_digit());

    if empieza_con_numero {
        limpia = "b".to_string() + &limpia;
    }

    if limpia == "id" || limpia == "ID" {
        limpia = "identificador".to_string();
    }

    limpia = limpia.replace("_1", "_01");

    let cond = limpia != cadena;

    ValidacionCadena { cadena: cadena.to_string(), sugerido: limpia, incidencia: cond }
    
}

static RE_NEWLINES: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[\r\n]+").unwrap());

static RE_SPACES: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\s+").unwrap());

fn texto_limpio(serie: &Series, fill_value: &str) -> Result<StringChunked, PolarsError> {
    let s = serie.cast(&DataType::String)?;
    let ca = s.str()?;

    let out = ca.apply(|opt| {
        let txt = opt.unwrap_or(fill_value);

        let txt = RE_NEWLINES.replace_all(txt, "; ");

        let txt = RE_SPACES.replace_all(&txt, " ");
        let txt = txt.trim().to_string();

        Some(txt.into())
    });

    Ok(out)
}

fn transformar_texto(serie: &Series) -> Result<Series, PolarsError> {
    let ca = texto_limpio(serie, "sin dato")?;

    Ok(ca.into_series())
}

fn texto_minusculas(serie: &Series) -> Result<Series, PolarsError> {
    let ca = texto_limpio(serie, "sin dato")?;

    let out = ca.apply(|opt| {
        Some(opt.unwrap().to_lowercase().into())
    });

    Ok(out.into_series())
}

fn texto_sin_guiones(serie: &Series) -> Result<Series, PolarsError> {
    let ca = texto_limpio(serie, "sin dato")?;

    let out = ca.apply(|opt| {
        Some(opt.unwrap().replace('_', " ").into())
    });

    Ok(out.into_series())
}

fn texto_capitalizado(serie: &Series) -> Result<Series, PolarsError> {
    let ca = texto_limpio(serie, "sin dato")?;

    let out = ca.apply(|opt| {
        let txt = opt.unwrap();
        let txt = txt.split_whitespace().map(|w| capitalize(w))
                            .collect::<Vec<_>>().join(" ");
        let txt = txt.replace(" De ", " de ").replace(" Del "," del ")
                            .replace(" Y "," y ").replace(" El "," el ")
                            .replace(" La ", " la ").replace(" A ", " a ")
                            .replace(" En ", " en ");

        Some(txt.into())
    });

    Ok(out.into_series())
}

fn capitalize(word: &str) -> String {
    let temp = word.to_lowercase();
    let mut c = temp.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str()
    }
    
}

fn anonimizar(serie: &Series) -> Result<Series, PolarsError> {
    let ca = texto_limpio(serie, "sin dato")?;

    let out = ca.apply(|opt| {
        let txt = opt.unwrap();
        if txt == "sin dato" {
            Some("sin dato".into())
        } else {
            let mut hasher = Sha256::new();
            hasher.update(txt.as_bytes());
            let hash = hasher.finalize();

            Some(hex::encode(hash).into())
        }
    });

    Ok(out.into_series())
}

const NULOS: &[&str] = &[
    "", "-", " ","NaT", "NA", "N/A", "ND", "nd", "*", "na", "nan", "null", "None",
];

static RE_MONEDA: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[$€]").unwrap());

fn limpiar_numericas(serie: &Series) -> Result<Series, PolarsError> {
    let s = serie.cast(&DataType::String)?;
    let ca = s.str()?;

    let limpio = ca.apply(|opt| {
        let txt = opt.unwrap_or("");
        let txt = txt.trim();

        let txt = RE_MONEDA.replace_all(txt, "");
        let txt = txt.replace(",", "");

        if NULOS.contains(&txt.as_ref()) {None} else {Some(txt.into())}
    });

    Ok(limpio.into_series())
}

fn limpiar_coordenadas(serie: &Series) -> Result<Series, PolarsError> {
    let s = serie.cast(&DataType::String)?;
    let ca = s.str()?;

    let limpio = ca.apply(|opt| {
        let txt = opt.unwrap_or("");
        let txt = txt.trim();

        if NULOS.contains(&txt.as_ref()) {Some("0.0".into())} else {Some(txt.into())}
    });

    Ok(limpio.into_series())
}

fn limpiar_fechas(serie: &Series) -> Result<Series, PolarsError> {
    let s = serie.cast(&DataType::String)?;
    let ca = s.str()?;

    let limpio = ca.apply(|opt| {
        let txt = opt.unwrap_or("");
        let txt = txt.trim();

        if NULOS.contains(&txt.as_ref()) {None} else {Some(txt.into())}
    });

    Ok(limpio.into_series())
}

fn transformar_a_numerica(serie: &Series) -> Result<Series, PolarsError> {
    let limpia = limpiar_numericas(serie)?;

    limpia.strict_cast(&DataType::Float64)
}

fn transformar_a_coordenada(serie: &Series) -> Result<Series, PolarsError> {
    let limpia = limpiar_coordenadas(serie)?;

    limpia.strict_cast(&DataType::Float64)
}

fn transformar_a_fecha(serie: &Series) -> Result<Series, PolarsError> {
    let limpia = limpiar_fechas(serie)?;
    let name = limpia.name().to_string();
    let height = limpia.len();
    let vcols = vec![limpia.into_column()];
    
    let df_temp = DataFrame::new(height, vcols)?;

    for formato in ["%Y-%m-%d", "%d-%m-%Y", "%Y/%m/%d", "%d/%m/%Y"] {
        let resultado = df_temp.clone().lazy()
            .with_column(
                col(name.as_str()).str().to_date(StrptimeOptions {
                    format: Some(formato.into()),
                    strict: true,
                    exact: true,
                    cache: true,
                })
            )
            .collect();

        match resultado {
            Ok(df_resultado) => return Ok(df_resultado.column(name.as_str())?.as_materialized_series().clone()),
            Err(_) => continue,
        }
    }

    Err(PolarsError::ComputeError("No se pudo convertir a fecha".into()))
}

fn nombre_archivo_sugerido(ruta: &str) -> String {
    let stem = Path::new(ruta)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("archivo");

    let limpio = validar_cadena(stem).sugerido;

    format!("{}.csv", limpio)
}

fn escribir_csv(df: &DataFrame, ruta: &str) -> Result<(), String> {

    let mut archivo = File::create(&ruta)
        .map_err(|e| format!("No se pudo crear archivo: {}", e))?;

    archivo
        .write_all(b"\xEF\xBB\xBF")
        .map_err(|e| e.to_string())?;


    CsvWriter::new(&mut archivo)
        .include_header(true)
        .finish(&mut df.clone())
        .map_err(|e| format!("Error al exportar CSV: {}", e))?;

    Ok(())
}

fn ruta_cache() -> Result<PathBuf, String> {
    let mut path = dirs::cache_dir().ok_or("No se pudo localizar el directorio de cache")?;

    path.push("validador_app");
    std::fs::create_dir_all(&path).map_err(|e| e.to_string())?;

    path.push("instituciones.json");

    Ok(path)
}

fn cargar_cache() -> CacheInstituciones {
    let ruta = match ruta_cache() {
        Ok(x) => x,
        Err(_) => return CacheInstituciones::default()
    };

    if !ruta.exists() {
        return CacheInstituciones::default();
    }

    let contenido = match std::fs::read_to_string(ruta) {
        Ok(x) => x,
        Err(_) => return CacheInstituciones::default(),
    };

    serde_json::from_str(&contenido).unwrap_or_default()
}

fn guardar_cache(cache: &CacheInstituciones) -> Result<(), String> {
    let ruta = ruta_cache()?;

    let json = serde_json::to_string_pretty(cache).map_err(|e| e.to_string())?;

    std::fs::write(ruta, json).map_err(|e| e.to_string())?;

    Ok(())
}

async fn obtener_lista_instituciones(cliente: &reqwest::Client) -> Result<Vec<String>, String> {

    let respuesta= cliente.get("https://www.datos.gob.mx/api/3/action/organization_list").send().await.map_err(|e| e.to_string())?.error_for_status().map_err(|e| e.to_string())?;

    let datos: CkanResponse<Vec<String>> = respuesta.json().await.map_err(|e| e.to_string())?;

    if !datos.success {return Err("CKAN regresó success=false".to_string());}

    Ok(datos.result)
}

async fn obtener_detalle_institucion(nombre: &str, cliente: &reqwest::Client) -> Result<Institucion, String> {

    let respuesta = cliente
    .get(
        "https://www.datos.gob.mx/api/3/action/organization_show"
    )
    .query(&[("id", nombre)]).send().await.map_err(|e| e.to_string())?.error_for_status().map_err(|e| e.to_string())?;

    let datos: CkanResponse<Institucion> =  respuesta.json().await.map_err(|e| e.to_string())?;

    if !datos.success {return Err("CKAN regresó success=false".to_string());}

    Ok(datos.result)
}

async fn obtener_conjuntos_institucion(institucion: &str, cliente: &reqwest::Client) -> Result<Vec<Conjunto>, String> {
    let respuesta = cliente
    .get(
        "https://www.datos.gob.mx/api/3/action/package_search"
    )
    .query(&[
        ("fq",format!("organization:{institucion}")),("rows","1000".to_string())
    ]).send().await.map_err(|e| e.to_string())?.error_for_status().map_err(|e| e.to_string())?;

    let datos: CkanResponse<PackageSearchResponse> = respuesta.json().await.map_err(|e| e.to_string())?;

    if !datos.success {return Err("CKAN regresó success=false".to_string());}

    let mut conjuntos = datos.result.results;

    for conjunto in &mut conjuntos {
        conjunto.institucion = institucion.to_string();
    }

    Ok(conjuntos)
}

struct ClientHandler;

impl russh::client::Handler for ClientHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &russh::keys::ssh_key::PublicKey,
    ) -> Result<bool, Self::Error>
    {
        Ok(true)
    }
}

async fn destruir(repo: RepoConexion) {

    if let Err(e) = repo.sftp.close().await {
        eprintln!("Error cerrando SFTP: {e}");
    }

    if let Err(e) = repo.ssh
        .disconnect(
            russh::Disconnect::ByApplication,
            "Operación completada",
            "es-MX",
        )
        .await
    {
        eprintln!("Error cerrando SSH: {e}");
    }
}

async fn subir_archivo_repo(repo: &RepoConexion, ruta_local: &PathBuf, ruta_remota: &str) -> Result<SubidaRepo, String> {
    
    let metadata = std::fs::metadata(&ruta_local)
    .map_err(|e| e.to_string())?;
    
    let mut local = tokio::fs::File::open(ruta_local).await.map_err(|e| e.to_string())?;

    let mut remoto = repo.sftp
    .open_with_flags(ruta_remota,
        OpenFlags::CREATE
            | OpenFlags::TRUNCATE
            | OpenFlags::WRITE,
    )
    .await
    .map_err(|e| format!("Error creando archivo remoto: {:?}", e))?;

    let bytes = tokio::io::copy(
    &mut local,
    &mut remoto,
    )
    .await
    .map_err(|e| e.to_string())?;

    drop(local);
    let _ = remoto.shutdown().await;

    let url = ruta_publica(ruta_remota);

    let subido = SubidaRepo {
        size_original: metadata.len(),
        size_subido: bytes,
        url,
    };

    Ok(subido)
}

fn ruta_publica(ruta_remota: &str) -> String {
    ruta_remota.replace("/mnt/nfs/datasets/","https://repodatos.atdt.gob.mx/")
}

async fn crear_ruta_remota(repo: &RepoConexion, institucion: &str, conjunto: &str) -> Result<String,String> {

    let base = "/mnt/nfs/datasets/api_update/".to_string();
    let inst_folder = base.clone() + institucion;
    let conj_folder = inst_folder.clone() + "/" + conjunto;

    crear_ruta(repo, &inst_folder).await?;
    crear_ruta(repo, &conj_folder).await?;

    let ruta_remota = conj_folder + "/";

    Ok(ruta_remota)
}

async fn crear_ruta(repo: &RepoConexion, ruta: &str) -> Result<(), String> {
    let cond = repo.sftp.try_exists(ruta).await.map_err(|e|format!("No se pudo corroborar si existe el directorio {e}"))?;
    if !cond {
        repo.sftp.create_dir(ruta).await.map_err(|e|format!("No se pudo crear el directorio{e}"))?;
    };

    Ok(())
}

fn exportar_temporal(df: DataFrame, ruta: &str) -> Result<PathBuf, String> {
    let ruta_temporal = ruta_csv_temporal(ruta)?;
    escribir_csv(&df, ruta_temporal.to_str().unwrap())?;
    Ok(ruta_temporal)
}

async fn conectar_repo(host: &str, usuario: &str) -> Result<RepoConexion, String> {
    let config = Arc::new(russh::client::Config::default());
    let mut ssh = russh::client::connect(config, host, ClientHandler)
        .await
        .map_err(|e| format!("Error de conexión SSH: {:?}", e))?;

    let home = dirs::home_dir().ok_or("No hay HOME")?;
    let llave_path = home.join(".ssh").join("id_ed25519");

    if !llave_path.exists() {
        return Err("No se encontró la llave SSH válida".to_string());
    };

    let key_pair = russh::keys::load_secret_key(llave_path, None)
        .map_err(|e| format!("Error al leer/parsear la llave privada: {:?}", e))?;

    let key_with_alg = russh::keys::key::PrivateKeyWithHashAlg::new(Arc::new(key_pair), None);

    let autenticado = ssh
        .authenticate_publickey(usuario, key_with_alg.into())
        .await
        .map_err(|e| format!("Error en el handshake de autenticación: {:?}", e))?;

    if !autenticado.success() {
        return Err("La autenticación por llave pública fue rechazada por el servidor".to_string());
    }

    let canal_sftp = ssh.channel_open_session().await.map_err(|e| format!("No se pudo abrir la sesión: {:?}", e))?;

    canal_sftp.request_subsystem(true, "sftp").await
    .map_err(|e| format!("Error iniciando subsistema SFTP: {:?}", e))?;

    let sftp = SftpSession::new(
        canal_sftp.into_stream(),
    )
    .await
    .map_err(|e| format!("Error creando sesión SFTP: {:?}", e))?;

    let conexion = RepoConexion {ssh,sftp};

    Ok(conexion)
}

fn ruta_csv_temporal(nombre: &str) -> Result<PathBuf, String> {
    let mut ruta = env::temp_dir();
    ruta.push("validador_app");

    std::fs::create_dir_all(&ruta)
        .map_err(|e| e.to_string())?;

    ruta.push(nombre);
    Ok(ruta)
}

fn limpiar_temporales() -> Result<(), String> {
    let mut ruta = env::temp_dir();
    ruta.push("validador_app");

    if ruta.exists() {
        std::fs::remove_dir_all(&ruta)
            .map_err(|e| e.to_string())?;
    }

    std::fs::create_dir_all(&ruta)
        .map_err(|e| e.to_string())?;

    Ok(())
}

pub async fn worker_subidas(cola: Arc<ColaSubidas>, app: tauri::AppHandle,) {
    loop {


        let trabajo = {
            let mut pendientes = cola.pendientes.lock().unwrap();
            pendientes.pop_front()
        };

        match trabajo {
            Some(mut job) => {
                job.estado = EstadoSubida::Subiendo;

                {
                    let mut actual = cola.en_proceso.lock().unwrap();
                    *actual = Some(job.clone());
                }

                let resultado = procesar_subida(&mut job).await;

                match resultado {
                    Ok(_) => {
                        job.estado = EstadoSubida::Completado;
                        let _ = std::fs::remove_file(&job.archivo_local);

                        let _ = app.emit("cola-actualizada", &job);
                    }

                    Err(e) => {
                        job.estado = EstadoSubida::Error;
                        job.error = Some(e);

                        let _ = app.emit("cola-actualizada", &job);
                    }
                }

                {
                    let mut actual = cola.en_proceso.lock().unwrap();
                    *actual = None;
                }

                {
                    let mut historico = cola.historico.lock().unwrap();
                    historico.push(job);
                }
            }

            None => {
                cola.notify.notified().await;
            }
        }

    }
}

async fn procesar_subida(job: &mut TrabajoSubida) -> Result<(), String> {

    let repo = conectar_repo(&job.host, &job.usuario).await
    .map_err(|e|format!("No se pudo conectar al repo {e}"))?;

    let resultado = async {

        let ruta_remota =
            crear_ruta_remota(
                &repo,
                &job.institucion,
                &job.conjunto,
            )
            .await?;

        let destino =
            ruta_remota
            + &job.archivo_remoto;

        let subida = subir_archivo_repo(
            &repo,
            &PathBuf::from(&job.archivo_local),
            &destino,
        )
        .await?;

        job.resultado = Some(subida);

        Ok::<(), String>(())

    }.await;

    destruir(repo).await;

    resultado
}