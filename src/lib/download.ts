import { base64ToBytes } from "@/lib/base64";

export function downloadBase64File(filename: string, dataBase64: string, mimeType: string) {
  const blob = new Blob([base64ToBytes(dataBase64)], { type: mimeType });
  const url = URL.createObjectURL(blob);
  const link = window.document.createElement("a");
  link.href = url;
  link.download = filename;
  window.document.body.appendChild(link);
  link.click();
  link.remove();
  window.setTimeout(() => URL.revokeObjectURL(url), 1_000);
}
