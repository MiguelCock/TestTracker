import { invoke } from "@tauri-apps/api/core";

export class IpcError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "IpcError";
  }
}

export async function ipc<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    const message =
      typeof error === "string"
        ? error
        : error instanceof Error
          ? error.message
          : "An unexpected error occurred.";
    if (/(locked|expired)/i.test(message)) {
      window.dispatchEvent(new Event("testtracker:session-locked"));
    }
    throw new IpcError(message);
  }
}
