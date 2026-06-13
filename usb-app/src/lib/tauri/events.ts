import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { isTauriAvailable } from '$lib/tauri/env';

/** Listen to a Tauri event; returns null in browser preview mode. */
export async function listenWhenTauri<T>(
  event: string,
  handler: (payload: T) => void
): Promise<UnlistenFn | null> {
  if (!isTauriAvailable()) return null;
  return listen<T>(event, (e) => handler(e.payload));
}
