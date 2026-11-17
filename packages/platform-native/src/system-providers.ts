import { DATE_PIPE_DEFAULT_TIMEZONE } from '@angular/common';
import {
  EnvironmentProviders,
  ErrorHandler,
  Injectable,
  LOCALE_ID,
  makeEnvironmentProviders,
  signal,
} from '@angular/core';
import { EVENT_MANAGER_PLUGINS, EventManagerPlugin } from '@angular/platform-browser';
import { ensureNgNativeBridge } from './bridge';

export interface NativeKeyEventPayload {
  key?: string;
  ctrlKey?: boolean;
  shiftKey?: boolean;
  altKey?: boolean;
  metaKey?: boolean;
}

function normalizeKeyToken(raw: string): string {
  const lower = raw.trim().toLowerCase();
  if (lower === 'esc') return 'escape';
  if (lower === 'spacebar' || lower === ' ') return 'space';
  if (lower === 'return') return 'enter';
  return lower;
}

/**
 * Matches Angular template keyboard shorthand syntax (e.g. `keydown.control.f`,
 * `keydown.escape`, `keydown.alt.arrowleft`) against native OS key event payloads.
 */
export function matchesKeyboardEvent(eventName: string, payload: NativeKeyEventPayload): boolean {
  const parts = eventName.trim().toLowerCase().split('.');
  if (parts.length < 2 || (parts[0] !== 'keydown' && parts[0] !== 'keyup')) {
    return false;
  }

  const targetKey = normalizeKeyToken(parts[parts.length - 1] ?? '');
  const actualKey = normalizeKeyToken(payload.key ?? '');
  if (!targetKey || actualKey !== targetKey) {
    return false;
  }

  const modifiers = new Set(parts.slice(1, -1));
  const wantCtrl = modifiers.has('control') || modifiers.has('ctrl');
  const wantShift = modifiers.has('shift');
  const wantAlt = modifiers.has('alt');
  const wantMeta = modifiers.has('meta') || modifiers.has('cmd') || modifiers.has('super');

  return (
    wantCtrl === Boolean(payload.ctrlKey) &&
    wantShift === Boolean(payload.shiftKey) &&
    wantAlt === Boolean(payload.altKey) &&
    wantMeta === Boolean(payload.metaKey)
  );
}

/**
 * Angular `EventManagerPlugin` translating `(keydown.<mod>.<key>)` and `(keyup.<mod>.<key>)`
 * bindings onto native `GtkShortcutController` / `GtkEventControllerKey` listeners.
 */
@Injectable()
export class NativeKeyEventsPlugin extends EventManagerPlugin {
  private readonly bridge = ensureNgNativeBridge();

  constructor() {
    super(null);
  }

  override supports(eventName: string): boolean {
    const lower = eventName.trim().toLowerCase();
    return (lower.startsWith('keydown.') || lower.startsWith('keyup.')) && lower.length > 8;
  }

  override addEventListener(
    element: unknown,
    eventName: string,
    handler: (event: unknown) => void,
  ): () => void {
    const baseEvent = eventName.trim().toLowerCase().startsWith('keyup.') ? 'keyup' : 'keydown';
    const nodeId =
      typeof element === 'string'
        ? element
        : typeof element === 'object' && element !== null && 'id' in element
          ? String((element as { id: unknown }).id)
          : 'win';

    return this.bridge.listen(nodeId, baseEvent, (rawEvent) => {
      const payload = (rawEvent ?? {}) as NativeKeyEventPayload;
      if (matchesKeyboardEvent(eventName, payload)) {
        handler(rawEvent);
      }
    });
  }
}

/**
 * Angular `ErrorHandler` forwarding unhandled exceptions into the Rust OS system log
 * (`journald` / `os_log` / Windows Event Log) and updating a reactive `lastError` Signal.
 */
@Injectable({ providedIn: 'root' })
export class NativeSystemErrorHandler implements ErrorHandler {
  private readonly bridge = ensureNgNativeBridge();
  readonly lastError = signal<string | null>(null);

  handleError(error: unknown): void {
    const message = error instanceof Error ? `${error.name}: ${error.message}` : String(error);
    this.lastError.set(message);
    this.bridge.logSystemError(message);
  }
}

export function provideSystemLocale(): EnvironmentProviders {
  return makeEnvironmentProviders([
    { provide: LOCALE_ID, useFactory: () => ensureNgNativeBridge().getSystemLocale() },
    {
      provide: DATE_PIPE_DEFAULT_TIMEZONE,
      useFactory: () => ensureNgNativeBridge().getSystemTimezone(),
    },
  ]);
}

export function provideNativeErrorHandler(): EnvironmentProviders {
  return makeEnvironmentProviders([
    NativeSystemErrorHandler,
    { provide: ErrorHandler, useExisting: NativeSystemErrorHandler },
  ]);
}

export function provideNativeKeyEvents(): EnvironmentProviders {
  return makeEnvironmentProviders([
    { provide: EVENT_MANAGER_PLUGINS, useClass: NativeKeyEventsPlugin, multi: true },
  ]);
}
