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
