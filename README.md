# Tauri Plugin vicons

A simple plugin that allows you to get the operating system icons by name (only works on Linux) by getting it in base64 ready to use the src of any image

## Installation

```bash
bun add @vasakgroup/plugin-vicons
```

Add in `cargo.toml`

```toml
[dependencies]
tauri-plugin-vicons = { git = "https://github.com/Vasak-OS/tauri-plugin-vicons", branch = "v2" }
```
In `main.rs` or `lib.rs`, add the following to your `tauri::Builder`:

```rust
use tauri_plugin_vicons;
fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_vicons::init()) // this line
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

And add in `src-tauri/compatibilites/default.json`

```json
{
  "permissions": [
    ...
    "vicons:default",
  ]
}

```

## Usage

```ts
import { getIconSource } from '@vasakgroup/plugin-vicons';

const icon = await getIconSource('folder');
```

in vue

```vue
<script setup lang="ts">
import { getIconSource } from '@vasakgroup/plugin-vicons';
import { ref } from 'vue';
const icon = ref('');
const getIcon = async () => {
  icon.value = await getIconSource('folder');
};
getIcon();
</script>

<template>
  <img :src="icon" />
</template>
```
