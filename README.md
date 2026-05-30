# Tauri Plugin vicons

Obtiene íconos nativos del sistema Linux (GTK) como base64, listos para usar en cualquier elemento `<img>`. Soporta temas de iconos del sistema, cacheo automático, y detección de cambios de tema en vivo.

## Requisitos

- Rust **1.80.0+**
- Tauri **v2**
- Linux con **GTK 3** (entornos GNOME, KDE, Xfce, etc.)

## Instalación

### 1. Agregar el crate Rust

```toml
[dependencies]
tauri-plugin-vicons = { git = "https://github.com/Vasak-OS/tauri-plugin-vicons", branch = "v2" }
```

### 2. Registrar el plugin en `lib.rs`

```rust
fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_vicons::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### 3. Agregar el paquete JS (opcional)

```bash
bun add @vasakgroup/plugin-vicons
```

### 4. Configurar permisos

En `src-tauri/capabilities/default.json`:

```json
{
  "permissions": [
    "vicons:default"
  ]
}
```

## API

### Comandos Rust

| Comando      | Descripción                                    | Retorno                  |
|-------------|------------------------------------------------|--------------------------|
| `get_icon`  | Obtiene un icono regular por nombre o ruta     | `String` (base64)        |
| `get_symbol`| Obtiene un icono simbólico por nombre o ruta   | `String` (base64)        |

Ambos aceptan un solo argumento `name: &str`. Si es una **ruta de archivo válida**, leen el archivo directamente. Si es un **nombre de icono GTK**, lo buscan en el tema de iconos del sistema.

### Eventos emitidos

| Evento                    | Cuándo se emite                          | Payload |
|---------------------------|------------------------------------------|---------|
| `vicons:theme-changed`    | El tema de iconos del sistema cambió     | `null`  |

Escucharlo desde el frontend:

```typescript
import { listen } from "@tauri-apps/api/event";

await listen("vicons:theme-changed", () => {
  // recargar íconos, refrescar UI, etc.
});
```

### Funciones JS (guest-js)

| Función            | Descripción                                                     | Retorno                |
|-------------------|-----------------------------------------------------------------|------------------------|
| `getIconSource`   | Obtiene un data URI completo listo para `<img src>`             | `string`               |
| `getSymbolSource` | Igual que `getIconSource` pero con iconos simbólicos            | `string`               |
| `getIcon`         | Obtiene solo el base64 de un icono regular (sin data URI)       | `Promise<string>`      |
| `getSymbol`       | Obtiene solo el base64 de un icono simbólico (sin data URI)     | `Promise<string>`      |

`getIconSource` y `getSymbolSource` detectan automáticamente el **tipo MIME** del icono (PNG, JPEG, GIF, WebP, BMP, SVG) mediante magic bytes.

## Uso

### Básico

```typescript
import { getIconSource } from '@vasakgroup/plugin-vicons';

const icon = await getIconSource('folder');
// → "data:image/svg+xml;base64,PHN2ZyB4bWxucz0..."
```

### Con Vue

```vue
<script setup lang="ts">
import { getIconSource } from '@vasakgroup/plugin-vicons';
import { ref, onMounted } from 'vue';

const icon = ref('');
onMounted(async () => {
  icon.value = await getIconSource('folder');
});
</script>

<template>
  <img :src="icon" alt="folder" />
</template>
```

### Con React

```tsx
import { getIconSource } from '@vasakgroup/plugin-vicons';
import { useEffect, useState } from 'react';

function FolderIcon() {
  const [src, setSrc] = useState('');
  useEffect(() => {
    getIconSource('folder').then(setSrc);
  }, []);
  return <img src={src} alt="folder" />;
}
```

### Ruta de archivo directa

Si pasás una ruta de archivo existente, se lee directamente sin pasar por el tema GTK:

```typescript
const icon = await getIconSource('/usr/share/icons/hicolor/48x48/apps/firefox.png');
```

### Escuchar cambios de tema

El plugin detecta automáticamente cambios en el tema de iconos del sistema (a través de la señal `changed` de GTK) y emite un evento:

```typescript
import { listen } from "@tauri-apps/api/event";
import { getIconSource } from '@vasakgroup/plugin-vicons';

listen("vicons:theme-changed", async () => {
  console.log("Theme changed, refreshing icons...");
  const icon = await getIconSource('folder');
  // actualizar UI con el nuevo icono
});
```

## Arquitectura

[![](https://mermaid.ink/img/pako:eNqNkktu2zAQhq8y4CaL2pZqxy-hSBHnobwcGIjRApUNgZImEhGJFChKiWt72wP0Bj1Az1CguUlPUopK4CToolxxRv_M_3FGaxKKCIlDblNxHyZUKphPFhz0KcogljRP4FQKrpBHTbo-h16M6jwU_EaUMkSwQMc3qywQaZNZQrt9AJNn2XyV44dAWgcZjVkIwUphAbNr17qYnbiWe35qfcZgZk2mM-vmk7vcGR15KSu0N-xVTPcpHJVghm0NymOM9p6Uhu0N9ISGd6-Yj2sYv-7isyxPG2a_MNAm0zCfrH__kqWiUD3-SFlEP253LU5qxaZ4_LmBUy9FlEBlmLBKLN9quNiA6wVlEVKp8cCdX4IZRI3_Qu0azzPtGdIwQUiYeml49mwI76BiMeo1bODci7ASaaXtTdHyrZ4LEIAPOZM00hwXXu2eCnFX5vDn23cIaIGDfXONSyoj-rrRv6ZpuKeCMyXkzu7SYzrjm534WfO1meKVpx_LMVT-06rMBAoWc5q-4L0y4qkXpkilbyB8ph8pX8umRnbtYcbUf_8Jh6bouAmuod3Rk5nTUjLASg9yA0ekRWLJIuIoWWKLZCgzWodkXRctiHFYEEdfI7ylZaoWZMG3uiyn_IsQ2XOlFGWcEOeWpoWOyjyiCo8Z1aPbSTQZyiNRckWcrm1aEGdNHnTUG3QG4_77fq87Gg979qhFVsTZH3XsQbfftce9bn846nW3LfLVeNqd0XB_rM9QF9lj2x5u_wKHiDMz?type=png)](https://mermaid.live/edit#pako:eNqNkktu2zAQhq8y4CaL2pZqxy-hSBHnobwcGIjRApUNgZImEhGJFChKiWt72wP0Bj1Az1CguUlPUopK4CToolxxRv_M_3FGaxKKCIlDblNxHyZUKphPFhz0KcogljRP4FQKrpBHTbo-h16M6jwU_EaUMkSwQMc3qywQaZNZQrt9AJNn2XyV44dAWgcZjVkIwUphAbNr17qYnbiWe35qfcZgZk2mM-vmk7vcGR15KSu0N-xVTPcpHJVghm0NymOM9p6Uhu0N9ISGd6-Yj2sYv-7isyxPG2a_MNAm0zCfrH__kqWiUD3-SFlEP253LU5qxaZ4_LmBUy9FlEBlmLBKLN9quNiA6wVlEVKp8cCdX4IZRI3_Qu0azzPtGdIwQUiYeml49mwI76BiMeo1bODci7ASaaXtTdHyrZ4LEIAPOZM00hwXXu2eCnFX5vDn23cIaIGDfXONSyoj-rrRv6ZpuKeCMyXkzu7SYzrjm534WfO1meKVpx_LMVT-06rMBAoWc5q-4L0y4qkXpkilbyB8ph8pX8umRnbtYcbUf_8Jh6bouAmuod3Rk5nTUjLASg9yA0ekRWLJIuIoWWKLZCgzWodkXRctiHFYEEdfI7ylZaoWZMG3uiyn_IsQ2XOlFGWcEOeWpoWOyjyiCo8Z1aPbSTQZyiNRckWcrm1aEGdNHnTUG3QG4_77fq87Gg979qhFVsTZH3XsQbfftce9bn846nW3LfLVeNqd0XB_rM9QF9lj2x5u_wKHiDMz)

### Cache

- Dos cachés separadas: `ICON_CACHE` y `SYMBOL_CACHE` (`HashMap<String, CacheEntry>`).
- Cada entrada expira después de **30 minutos**.
- Al cambiar el tema de iconos del sistema, **ambos cachés se limpian por completo**.
- La caché usa `std::sync::LazyLock` y `std::sync::Mutex` (sin dependencias externas).

### Detección de cambios de tema

GTK monitorea internamente los cambios de tema mediante:

- **GSettings** (`org.gnome.desktop.interface icon-theme`)
- **inotify** sobre `~/.config/gtk-3.0/settings.ini`
- **XDG Desktop Portal**

Cuando detecta un cambio, dispara la señal `changed` del `IconTheme`. El plugin la captura, limpia la caché, y emite `vicons:theme-changed` al frontend.

### Logging

El plugin escribe logs en `{app_data_dir}/logs/icons.log` con niveles INFO, WARN y ERROR. Sin dependencias externas de logging — usa `std::io::LineWriter` directo a archivo.

## Errores

| Error               | Causa                                       |
|---------------------|---------------------------------------------|
| `IconNotFound`      | El nombre solicitado no existe en el tema   |
| `ThemeMonitorError` | No se pudo inicializar el monitor de tema   |
| `Io`                | Error de lectura de archivo                 |

## Dependencias

Solo 5 crates además de Tauri:

| Crate       | Propósito                     |
|------------|-------------------------------|
| `serde`    | Serialización                 |
| `thiserror`| Errores tipados               |
| `gtk`      | Acceso al theme de iconos GTK |
| `glib`     | Bindings de GLib (con GTK)    |
| `base64`   | Codificación base64           |

## Licencia

GPLv3 — Vasak Group
