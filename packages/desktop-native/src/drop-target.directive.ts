import { Directive, ElementRef, inject, OnDestroy, output, Renderer2 } from '@angular/core';

export interface NativeFileDropPayload {
  value?: string;
  paths?: string[];
}

/**
 * Standalone directive (`[ngNativeDropTarget]`) that enables OS-native file drag-and-drop
 * (`GtkDropTarget` on Linux GTK4, `AllowDrop` on WinUI 3, `registerForDraggedTypes` on macOS AppKit)
 * and emits dropped file paths through `(fileDrop)`.
 */
@Directive({
  selector: '[ngNativeDropTarget]',
})
export class NativeDropTargetDirective implements OnDestroy {
  readonly fileDrop = output<string[]>();
  private readonly unlisten: () => void;

  constructor() {
    const renderer = inject(Renderer2);
    const elRef = inject<ElementRef<unknown>>(ElementRef);
    const nativeEl = elRef.nativeElement;

    renderer.setAttribute(nativeEl, 'drop-target', 'true');
    this.unlisten = renderer.listen(nativeEl, 'drop', (event?: unknown) => {
      const paths = NativeDropTargetDirective.extractPaths(event);
      if (paths.length > 0) {
        this.fileDrop.emit(paths);
      }
    });
  }

  static extractPaths(event: unknown): string[] {
    if (typeof event === 'string') {
      return event
        .split(/\r?\n|,/)
        .map((p) => p.trim())
        .filter((p) => p.length > 0);
    }
    if (event && typeof event === 'object') {
      const payload = event as NativeFileDropPayload;
      if (Array.isArray(payload.paths)) {
        return payload.paths.map((p) => String(p).trim()).filter((p) => p.length > 0);
      }
      if (typeof payload.value === 'string') {
        return payload.value
          .split(/\r?\n|,/)
          .map((p) => p.trim())
          .filter((p) => p.length > 0);
      }
    }
    return [];
  }

  ngOnDestroy(): void {
    this.unlisten();
  }
}
