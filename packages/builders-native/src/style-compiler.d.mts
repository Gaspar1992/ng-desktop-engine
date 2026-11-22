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
}

export function parseStylesCss(cssSource: string, from?: string): StyleRuleIr[];

export function formatStyleRuleExtensions(rule: StyleRuleIr): string;

export function emitStyleInstruction(rule: StyleRuleIr): string;
