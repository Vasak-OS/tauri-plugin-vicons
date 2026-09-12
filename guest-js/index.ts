import { invoke } from "@tauri-apps/api/core";

// Get icon from Linux in base64
async function getIcon(name: string): Promise<string> {
  try {
    return await invoke("plugin:vicons|get_icon", { name });
  } catch (error) {
    console.error("[Icon Error] Error obteniendo icono:", error);
  }
  return "";
}

// Get Symbol from Linux in base64
async function getSymbol(name: string): Promise<string> {
  try {
    return await invoke("plugin:vicons|get_symbol", { name });
  } catch (error) {
    console.error("[Icon Error] Error obteniendo simbolo:", error);
  }
  return "";
}

/**
 * Si el tema puede dibujar este nombre.
 *
 * Hace falta porque `getIcon` y `getSymbol` no fallan cuando el icono no está:
 * el tema devuelve `image-missing` —el cuadrito de imagen rota— como si fuera el
 * icono pedido, y lo que llega acá es un base64 válido, indistinguible del de
 * verdad. Sin esto, la única forma de detectarlo era pedir `image-missing` a
 * propósito y comparar.
 *
 * Ante un error del plugin contesta `false`, igual que el resto del módulo
 * contesta `""`: quien pregunta esto lo hace para elegir un icono alternativo, y
 * caer al alternativo es lo mismo que hacía hasta ahora. El error queda en la
 * consola.
 */
async function has(command: "has_icon" | "has_symbol", name: string): Promise<boolean> {
  try {
    return await invoke(`plugin:vicons|${command}`, { name });
  } catch (error) {
    console.error("[Icon Error] Error preguntando por el icono:", error);
  }
  return false;
}

/** Si `getIconSource` va a devolver este icono y no el cuadrito. */
export async function hasIcon(name: string): Promise<boolean> {
  return has("has_icon", name);
}

/** Si `getSymbolSource` va a devolver este símbolo y no el cuadrito. */
export async function hasSymbol(name: string): Promise<boolean> {
  return has("has_symbol", name);
}

function getIconType(base64String: string): string {
  try {
    const binaryString = atob(base64String.substring(0, 44));
    const bytes = new Uint8Array(binaryString.length);
    for (let i = 0; i < binaryString.length; i++) {
      bytes[i] = binaryString.charCodeAt(i);
    }

    if (bytes[0] === 0x89 && bytes[1] === 0x50 && bytes[2] === 0x4e && bytes[3] === 0x47)
      return "image/png";

    if (bytes[0] === 0xff && bytes[1] === 0xd8 && bytes[2] === 0xff)
      return "image/jpeg";

    if (bytes[0] === 0x47 && bytes[1] === 0x49 && bytes[2] === 0x46 && bytes[3] === 0x38)
      return "image/gif";

    if (bytes[0] === 0x52 && bytes[1] === 0x49 && bytes[2] === 0x46 && bytes[3] === 0x46)
      return "image/webp";

    if (bytes[0] === 0x42 && bytes[1] === 0x4d)
      return "image/bmp";

    return "image/svg+xml";
  } catch {
    return "image/svg+xml";
  }
}

export async function getIconSource(value: string): Promise<string> {
  try {
    const icon = await getIcon(value);
    if (!icon) return "";
    return `data:${getIconType(icon)};base64,${icon}`;
  } catch (error) {
    console.error("[Icon Error] Error obteniendo icono:", error);
    return "";
  }
}

export async function getSymbolSource(value: string): Promise<string> {
  try {
    const symbol = await getSymbol(value);
    if (!symbol) return "";
    return `data:${getIconType(symbol)};base64,${symbol}`;
  } catch (error) {
    console.error("[Icon Error] Error obteniendo simbolo:", error);
    return "";
  }
}
