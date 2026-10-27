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

- [x64-nsis](https://github.com/irvingfisica/validador_app/releases/download/v0.8.0/validador_1.0.8_x64-setup.exe)

- [x64-msi](https://github.com/irvingfisica/validador_app/releases/download/v0.8.0/validador_1.0.8_x64_en-US.msi)

### Mac:

- [x64-dmg](https://github.com/irvingfisica/validador_app/releases/download/v0.8.0/validador_1.0.8_x64.dmg)


### Linux:

- [x86_64-rpm](https://github.com/irvingfisica/validador_app/releases/download/v0.8.0/validador-1.0.8-1.x86_64.rpm)

- [amd64-AppImage](https://github.com/irvingfisica/validador_app/releases/download/v0.8.0/validador_1.0.8_amd64.AppImage)

- [amd64-deb](https://github.com/irvingfisica/validador_app/releases/download/v0.8.0/validador_1.0.8_amd64.deb)

## Compilar por tu cuenta

### Prerrequisitos
#### Tauri CLI
Para instalar Tauri CLi, consulta [la documentación](https://v2.tauri.app/start/prerequisites/)
para conocer los prerrequisitos necesarios según tu sistema operativo. Una vez hecho esto, puedes instalarla usando este comando:

```shell
npm install --save-dev @tauri-apps/cli@latest
```

#### Rust
Para instalar Rust en Mac o Linux, puedes usar:
```shell
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```
En caso de usar Windows, consulta [la documentación](https://rust-lang.org/tools/install/)

#### Vite
Para instalar Vite puedes usar:
```shell
npm install -D vite
```

### Compilación

1. Clonar el repositorio
```shell
git clone https://github.com/irvingfisica/validador_app.git
```

2. Abrir el proyecto
```shell
cd validador_app
```

3. Para levantar localmente el proyecto
```shell
npm run tauri dev
```

4. Para desplegarlo
```shell
npm run tauri build
```

5. Los ejecutables estarán en el directorio:
```shell
cd ./src-tauri/target/release/bundle
```