# Validador para PNDA

Este es la nueva versión de los validadores para la PNDA. Funcionan como aplicaciones independientes y es necesario instalarlas. 

El proyecto está construido en Rust usando Tauri. 

Actualemnte cuenta con 4 herramientas:

- Carga de archivos (detección de encoding y conversión a UTF8)
- Validación de columnas y tipos
- Edición de valores categóricos
- Exportar a CSV (siempre en UTF8 con BOM)

## Versiones instalables
Todas las versiones serán detectadas como inseguras por el sistema operativo pues no están firmadas.

### Windows:

- [x64-nsis](https://github.com/irvingfisica/validador_app/releases/download/untagged-4016b63b27de7641cd59/validador_0.1.0_x64-setup.exe)

- [x64-msi](https://github.com/irvingfisica/validador_app/releases/download/untagged-4016b63b27de7641cd59/validador_0.1.0_x64_en-US.msi)

### Mac:

- [aarch64-dmg](https://github.com/irvingfisica/validador_app/releases/download/untagged-4016b63b27de7641cd59/validador_0.1.0_aarch64.dmg) (Mac de las nuevas)

- [x64-dmg](https://github.com/irvingfisica/validador_app/releases/download/untagged-f39c84c73fa4c109c749/validador_0.1.0_x64.dmg) (Mac de las viejitas)


