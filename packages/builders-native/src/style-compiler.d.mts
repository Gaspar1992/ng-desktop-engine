export interface StyleRuleIr {
  className: string;
  background: string;
  color: string;
  padding: string;
  spacing: string;
  borderRadius: number | null;
  fontSize: number | null;
  fontWeight: number | null;
  minWidth: number | null;
  minHeight: number | null;
  orientation: 'horizontal' | 'vertical' | null;
  hexpand: boolean | null;
  vexpand: boolean | null;
  halign: string | null;
  valign: string | null;
  minChildren: number | null;
  maxChildren: number | null;
  homogeneous: boolean | null;
  pseudoClass: string | null;
  mediaTheme: 'light' | 'dark' | null;
  mediaMinWidth: number | null;
  mediaMaxWidth: number | null;
  opacity: number | null;
  transitionMs: number | null;
  transitionProp: string | null;
}

export interface SharedCssVariables {
  baseVars?: Map<string, string>;
  lightVars?: Map<string, string>;
  darkVars?: Map<string, string>;
}

/** Extracts `--var` custom properties from `:root` and `@media (prefers-color-scheme)` blocks. */
export function extractCssVariables(
  cssSource: string,
  from?: string,
  target?: SharedCssVariables,
): Required<SharedCssVariables>;

export function buildStyleRuleKey(rule: StyleRuleIr): string;

export function parseStylesCss(
  cssSource: string,
  from?: string,
  sharedVars?: SharedCssVariables | null,
): StyleRuleIr[];

export function formatStyleRuleExtensions(rule: StyleRuleIr): string;

export function emitStyleInstruction(rule: StyleRuleIr): string;
