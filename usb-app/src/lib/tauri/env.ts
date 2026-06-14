import { isTauri } from '@tauri-apps/api/core';

/** True when running inside the Tauri webview (not a plain browser tab). */
export function isTauriAvailable(): boolean {
  return isTauri();
}

export class TauriUnavailableError extends Error {
  constructor() {
    super(
      'Not running in the PatientVault desktop app. Start with `pnpm dev:usb` — do not open localhost:1420 in a regular browser.'
    );
    this.name = 'TauriUnavailableError';
  }
}

export function requireTauri(): void {
  if (!isTauriAvailable()) {
    throw new TauriUnavailableError();
  }
}
