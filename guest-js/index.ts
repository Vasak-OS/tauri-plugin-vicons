import { invoke } from "@tauri-apps/api/core";

// Get icon from Linux in base64
async function getIcon(value: string): Promise<string> {
  return invoke<{ value?: string }>("plugin:vicons|get_icon", {
    payload: {
      value,
    },
  }).then((r: any) => r.value);
}

// Get Symbol from Linux in base64
async function getSymbol(value: string): Promise<string> {
  return invoke<{ value?: string }>("plugin:vicons|get_symbol", {
    payload: {
      value,
    },
  }).then((r: any) => r.value);
}

// Get icon type from base64
// This function checks the first bytes of the base64 string to determine if it's a PNG or SVG
// It returns 'image/png' for PNG and 'image/svg+xml' for SVG
function getIconType(base64String: string): string {
  try {
    // Decode the first bytes of the base64
    const binaryString = atob(base64String.substring(0, 32));
    const bytes = new Uint8Array(binaryString.length);
    for (let i = 0; i < binaryString.length; i++) {
      bytes[i] = binaryString.charCodeAt(i);
    }
    // Verify if it's PNG (signature bytes)
    if (
      bytes[0] === 0x89 &&
      bytes[1] === 0x50 &&
      bytes[2] === 0x4e &&
      bytes[3] === 0x47
    ) {
      return "image/png";
    }

    // Default, assume SVG
    // Check for SVG signature bytes
    return "image/svg+xml";
  } catch (error) {
    console.error("Error identificando tipo de imagen:", error);
    return "image/png";
  }
}

export async function getIconSource(value: string): Promise<string> {
  try {
    const icon = await getIcon(value);
    return `data:${getIconType(icon)};base64,${icon}`;
  } catch (error) {
    console.error("[Icon Error] Error obteniendo icono:", error);
    return "";
  }
}

export async function getSymbolSource(value: string): Promise<string> {
  try {
    const symbol = await getSymbol(value);
    return `data:${getIconType(symbol)};base64,${symbol}`;
  } catch (error) {
    console.error("[Icon Error] Error obteniendo simbolo:", error);
    return "";
  }
}
