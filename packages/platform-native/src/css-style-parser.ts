import { NgNativeBridge } from './bridge';

export interface ParsedStyleRule {
  className: string;
  background: string;
  color: string;
  padding: number;
  spacing: number;
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

const RUNTIME_STYLE_EXTENSIONS: ReadonlyArray<readonly [keyof ParsedStyleRule, string]> = [
  ['borderRadius', 'radius'],
  ['fontSize', 'font-size'],
  ['fontWeight', 'font-weight'],
  ['minWidth', 'min-width'],
  ['minHeight', 'min-height'],
  ['orientation', 'orientation'],
  ['hexpand', 'hexpand'],
  ['vexpand', 'vexpand'],
  ['halign', 'halign'],
  ['valign', 'valign'],
  ['minChildren', 'min-children'],
  ['maxChildren', 'max-children'],
  ['homogeneous', 'homogeneous'],
];

export function parsePixelInt(raw: string): number | null {
  const parsed = Number.parseInt(raw.trim().replace(/px$/, ''), 10);
  return Number.isFinite(parsed) && parsed >= 0 ? parsed : null;
}

export function mapAlignKeyword(raw: string): string | null {
  switch (raw.trim().toLowerCase()) {
    case 'center':
      return 'center';
    case 'flex-start':
    case 'start':
      return 'start';
    case 'flex-end':
    case 'end':
      return 'end';
    case 'stretch':
    case 'fill':
      return 'fill';
    case 'baseline':
      return 'baseline';
    default:
      return null;
  }
}

function applyRuntimeGridColumns(rule: ParsedStyleRule, val: string): void {
  const match = val
    .trim()
    .match(/repeat\(\s*(auto-fit|auto-fill|\d+)\s*,\s*(?:minmax\(\s*(\d+)px\s*,[^)]+\)|[^)]+)\)/i);
  if (!match) return;
  const track = match[1].toLowerCase();
  if (track === 'auto-fit' || track === 'auto-fill') {
    rule.minChildren = 1;
    rule.maxChildren = 12;
  } else {
    const cols = Number.parseInt(track, 10);
    if (!Number.isFinite(cols) || cols < 1) return;
    rule.minChildren = cols;
    rule.maxChildren = cols;
  }
  rule.homogeneous = true;
  rule.hexpand = true;
  if (match[2] && rule.minWidth === null) {
    rule.minWidth = Number.parseInt(match[2], 10);
  }
}

function applyRuntimeLayoutDeclaration(rule: ParsedStyleRule, prop: string, val: string): void {
  const lower = val.trim().toLowerCase();
  if (prop === 'flex-direction') {
    if (lower.startsWith('row')) rule.orientation = 'horizontal';
    else if (lower.startsWith('column')) rule.orientation = 'vertical';
  } else if (prop === 'flex-wrap' && lower.startsWith('wrap')) {
    rule.minChildren ??= 1;
    rule.maxChildren ??= 12;
    rule.homogeneous = true;
    rule.hexpand = true;
  } else if (prop === 'flex' || prop === 'flex-grow') {
    const grow = Number.parseFloat(lower);
    if (Number.isFinite(grow) && grow >= 1) rule.hexpand = true;
    if (prop === 'flex') {
      const basisMatch = lower.match(/(\d+)px/);
      if (basisMatch && rule.minWidth === null) {
        rule.minWidth = Number.parseInt(basisMatch[1], 10);
      }
    }
  } else if (prop === 'width') {
    if (lower === '100%') rule.hexpand = true;
    else {
      const px = parsePixelInt(val);
      if (px !== null && rule.minWidth === null) rule.minWidth = px;
    }
  } else if (prop === 'height') {
    if (lower === '100%') rule.vexpand = true;
    else {
      const px = parsePixelInt(val);
      if (px !== null && rule.minHeight === null) rule.minHeight = px;
    }
  } else if (prop === 'grid-template-columns') {
    applyRuntimeGridColumns(rule, val);
  } else if (prop === 'justify-self') {
    const aligned = mapAlignKeyword(val);
    if (aligned) rule.halign = aligned;
  } else if (prop === 'align-self') {
    const aligned = mapAlignKeyword(val);
    if (aligned) rule.valign = aligned;
  } else if (prop === 'justify-content') {
    const aligned = mapAlignKeyword(val);
    if (!rule.hexpand && aligned && rule.halign === null) rule.halign = aligned;
  } else if (prop === 'align-items') {
    const aligned = mapAlignKeyword(val);
    if (!rule.vexpand && aligned && rule.valign === null) rule.valign = aligned;
  }
}

/**
 * Parses component CSS rules at runtime into `ParsedStyleRule` descriptors compatible with
 * the Rust engine's CSS Flexbox/Grid and typography/radius extension syntax.
 */
export function parseRuntimeStyles(cssSource: string): ParsedStyleRule[] {
  const cleaned = cssSource
    .replace(/\/\*[\s\S]*?\*\//g, '')
    .replace(/\[_ng(?:content|host)-[^\]]+\]/g, '');
  const rulesByClass = new Map<string, ParsedStyleRule>();
  const blockRe = /([^{}@]+)\{([^{}]*)\}/g;

  for (const match of cleaned.matchAll(blockRe)) {
    const selectorList = match[1];
    const body = match[2];
    const declarations = body.split(';');

    for (const rawSelector of selectorList.split(',')) {
      const clsMatch = rawSelector.trim().match(/^\.([a-zA-Z0-9_-]+)$/);
      if (!clsMatch) {
        continue;
      }
      const className = clsMatch[1];
      const rule: ParsedStyleRule = rulesByClass.get(className) ?? {
        className,
        background: 'none',
        color: 'none',
        padding: 0,
        spacing: 0,
        borderRadius: null,
        fontSize: null,
        fontWeight: null,
        minWidth: null,
        minHeight: null,
        orientation: null,
        hexpand: null,
        vexpand: null,
        halign: null,
        valign: null,
        minChildren: null,
        maxChildren: null,
        homogeneous: null,
      };

      for (const decl of declarations) {
        const colon = decl.indexOf(':');
        if (colon === -1) {
          continue;
        }
        const prop = decl.slice(0, colon).trim().toLowerCase();
        const val = decl.slice(colon + 1).trim();
        if (!val) {
          continue;
        }
        if (prop === 'background' || prop === 'background-color') {
          rule.background = val;
        } else if (prop === 'color') {
          rule.color = val;
        } else if (prop === 'padding') {
          const px = parsePixelInt(val);
          if (px !== null) rule.padding = px;
        } else if (prop === 'gap' || prop === 'spacing') {
          const px = parsePixelInt(val);
          if (px !== null) rule.spacing = px;
        } else if (prop === 'row-gap' || prop === 'column-gap') {
          if (rule.spacing === 0) {
            const px = parsePixelInt(val);
            if (px !== null) rule.spacing = px;
          }
        } else if (prop === 'border-radius') {
          const px = parsePixelInt(val);
          if (px !== null) rule.borderRadius = px;
        } else if (prop === 'font-size') {
          const px = parsePixelInt(val);
          if (px !== null) rule.fontSize = px;
        } else if (prop === 'font-weight') {
          const kw = val === 'bold' ? 700 : val === 'normal' ? 400 : parsePixelInt(val);
          if (kw !== null) rule.fontWeight = kw;
        } else if (prop === 'min-width') {
          const px = parsePixelInt(val);
          if (px !== null) rule.minWidth = px;
        } else if (prop === 'min-height') {
          const px = parsePixelInt(val);
          if (px !== null) rule.minHeight = px;
        } else {
          applyRuntimeLayoutDeclaration(rule, prop, val);
        }
      }
      rulesByClass.set(className, rule);
    }
  }

  return [...rulesByClass.values()];
}

/**
 * Formats optional `ParsedStyleRule` layout/typography fields as space-separated `key=value` tokens.
 */
export function formatParsedStyleExtensions(rule: ParsedStyleRule): string {
  const extensions: string[] = [];
  for (const [prop, key] of RUNTIME_STYLE_EXTENSIONS) {
    const value = rule[prop];
    if (value !== null && value !== undefined) {
      extensions.push(`${key}=${value}`);
    }
  }
  return extensions.join(' ');
}

export function registerParsedStyles(bridge: NgNativeBridge, styles: readonly string[]): void {
  for (const css of styles) {
    if (typeof css !== 'string' || css.trim().length === 0) {
      continue;
    }
    for (const rule of parseRuntimeStyles(css)) {
      bridge.registerStyle(
        rule.className,
        rule.background,
        rule.color,
        rule.padding,
        rule.spacing,
        formatParsedStyleExtensions(rule),
      );
    }
  }
}
