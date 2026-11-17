import { computed, Injectable, Signal, signal, WritableSignal } from '@angular/core';
import { ensureNgNativeBridge, NativeConnectivity } from '@ng-native/platform';

/**
 * Reactive OS Network Monitor backed by `GNetworkMonitor` (Linux GIO), `NWPathMonitor` (macOS),
 * or `NetworkInformation` (Windows), exposing Signal-based online/metered state.
 */
@Injectable({ providedIn: 'root' })
export class NativeNetworkService {
  private readonly bridge = ensureNgNativeBridge();
  readonly isOnline: WritableSignal<boolean>;
  readonly isMetered: WritableSignal<boolean>;
  readonly connectivity: Signal<NativeConnectivity>;

  constructor() {
    const initial = this.bridge.getNetworkStatus();
    this.isOnline = signal<boolean>(initial.online);
    this.isMetered = signal<boolean>(initial.metered);
    this.connectivity = computed<NativeConnectivity>(() => {
      if (!this.isOnline()) return 'offline';
      return this.isMetered() ? 'limited' : 'full';
    });
  }

  refresh(): void {
    const current = this.bridge.getNetworkStatus();
    this.isOnline.set(current.online);
    this.isMetered.set(current.metered);
  }

  setOnlineStatus(online: boolean, metered = false): void {
    this.bridge.setNetworkStatus(online, metered);
    this.isOnline.set(online);
    this.isMetered.set(metered);
  }
}
