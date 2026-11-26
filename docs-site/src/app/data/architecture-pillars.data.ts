export interface PipelineStage {
  readonly id: string;
  readonly step: string;
  readonly title: string;
  readonly summary: string;
  readonly modules: readonly string[];
  readonly detail: string;
}

export const PIPELINE_STAGES: readonly PipelineStage[] = [
  {
    id: 'angular-ivy',
    step: '01',
    title: 'Angular 22 Ivy & Signal Graph',
    summary:
      'Standard Angular components, Signals, Control Flow, and Signal Forms run untouched without any DOM polyfill.',
    modules: ['@angular/core@22', '@ng-desktop/platform', '@ng-desktop/forms'],
    detail:
      'Angular change detection emits fine-grained instructions into NativeRenderer2 rather than mutating an HTML DOM. Component templates compile ahead-of-time via @ng-desktop/builders or run cleanly through Ivy runtime factories.',
  },
  {
    id: 'ngb-compiler',
    step: '02',
    title: 'AOT .ngb Bytecode & CSS Compiler',
    summary:
      'Static templates, scoped ViewEncapsulation rules, and :root theme variables compile into compact .ngb instructions.',
    modules: ['packages/builders-native', 'oxc-parser', 'postcss'],
    detail:
      'Build-time style extraction parses Flexbox/Grid rules, pseudo-classes (:hover, :focus, :active, :disabled), opacity, transitions, and CSS custom property dark/light variants into pre-validated opcodes alongside SHA-256 asset hashing.',
  },
  {
    id: 'rust-js-runtime',
    step: '03',
    title: 'Dual JS Engine (JavaScriptCore / QuickJS-NG)',
    summary:
      'Embeds OS JavaScriptCore or QuickJS-NG in minimal RAM — no Chromium or Node child process required.',
    modules: ['src/runtime/quickjs_engine.rs', 'src/runtime/jsc_sys.rs'],
    detail:
      'Native host functions cross cleanly from JS into bounded Rust queues with microtask draining per frame.',
  },
  {
    id: 'arena-queue',
    step: '04',
    title: 'Generational Arena & Command Queue',
    summary:
      'Safe Rust tree ownership using generational NodeId handles avoids raw pointers, cycles, and main-thread stalls.',
    modules: ['src/engine/arena.rs', 'src/engine/commands.rs'],
    detail:
      'Every widget lives in a generational Arena (index + generation) with runtime BorrowState checking. UI mutations batch into RenderCommand frames and reconcile cleanly onto the OS UI toolkit.',
  },
  {
    id: 'layout-css',
    step: '05',
    title: 'Shared Layout Solver & Style Engine',
    summary:
      'Deterministic 1D Flexbox + 2D Grid geometry solver shared across Windows and macOS alongside GTK4 CSS.',
    modules: ['src/platforms/layout_solver.rs', 'src/engine/styles.rs'],
    detail:
      'Viewport resize, ng-clamp tightening, ng-paned split positions, and @media (prefers-color-scheme) token cascades compute consistent bounding rects in layout_solver.rs while GTK4 integrates natively via GSK GPU/CSS providers.',
  },
  {
    id: 'native-backends',
    step: '06',
    title: 'GTK4, Win32/WinUI, AppKit & OS Packaging',
    summary: 'Direct native OS widgets plus .deb, AppImage, Flatpak, .msix, and .dmg installers.',
    modules: ['src/platforms/', 'src/packager/builders.rs'],
    detail:
      'Linux GTK4 renders through GPU-accelerated GSK (Vulkan/GL), macOS AppKit composites via WindowServer Metal layers, Windows drives native DWM-composited HWNDs, and builders package native OS installers.',
  },
];

export interface FrameworkComparisonRow {
  readonly aspect: string;
  readonly ngDesktop: string;
  readonly electron: string;
  readonly tauri: string;
}

export const COMPARISON_ROWS: readonly FrameworkComparisonRow[] = [
  {
    aspect: 'Rendering Surface',
    ngDesktop: 'True GTK4 / WinUI 3 / AppKit widgets',
    electron: 'Bundled Chromium Blink per window',
    tauri: 'OS WebView2 / WebKitGTK / WKWebView',
  },
  {
    aspect: 'Baseline Memory (Idle)',
    ngDesktop: '~18–35 MB',
    electron: '~150–320 MB',
    tauri: '~60–110 MB',
  },
  {
    aspect: 'Minimum Binary Size',
    ngDesktop: '~4–12 MB (+ system toolkit)',
    electron: '~95–140 MB',
    tauri: '~6–15 MB',
  },
  {
    aspect: 'Angular Integration',
    ngDesktop: 'First-class Ivy Renderer2 + Signal Forms',
    electron: 'Manual IPC + DOM inside Chromium',
    tauri: 'Manual invoke() + WebView DOM',
  },
  {
    aspect: 'HTML/CSS Tax',
    ngDesktop: 'Zero WebView — compiled .ngb & native styles',
    electron: 'Full browser runtime + CSS compositor',
    tauri: 'Full WebView runtime + OS browser quirks',
  },
];
