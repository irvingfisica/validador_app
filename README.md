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

- [x64-nsis](https://github.com/irvingfisica/validador_app/releases/download/v0.7.0/validador_1.0.7_x64-setup.exe)

- [x64-msi](https://github.com/irvingfisica/validador_app/releases/download/v0.7.0/validador_1.0.7_x64_en-US.msi)

### Mac:

- [x64-dmg](https://github.com/irvingfisica/validador_app/releases/download/v0.7.0/validador_1.0.7_x64.dmg)


### Linux:

- [x86_64-rpm](https://github.com/irvingfisica/validador_app/releases/download/v0.7.0/validador-1.0.7-1.x86_64.rpm)

- [amd64-AppImage](https://github.com/irvingfisica/validador_app/releases/download/v0.7.0/validador_1.0.7_amd64.AppImage)

- [amd64-deb](https://github.com/irvingfisica/validador_app/releases/download/v0.7.0/validador_1.0.7_amd64.deb)

