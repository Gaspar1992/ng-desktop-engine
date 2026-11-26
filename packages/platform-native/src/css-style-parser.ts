import { NgNativeBridge } from './bridge';

export type SupportedMediaTheme = 'light' | 'dark';

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
  pseudoClass: string | null;
  mediaTheme: SupportedMediaTheme | null;
  mediaMinWidth: number | null;
  mediaMaxWidth: number | null;
  opacity: number | null;
  transitionMs: number | null;
  transitionProp: string | null;
}

interface MediaContext {
  mediaTheme: SupportedMediaTheme | null;
  mediaMinWidth: number | null;
  mediaMaxWidth: number | null;
  unsupported?: boolean;
}

interface ExtractedCssBlock {
  selectorText: string;
  body: string;
  media: MediaContext;
}

interface ParsedSelectorTarget {
  className: string;
  baseClass: string;
  scopeToken: string | null;
  pseudoClass: string | null;
}

interface ParsedStyleEntry {
  rule: ParsedStyleRule;
  baseClass: string;
  scopeToken: string | null;
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
  ['pseudoClass', 'pseudo'],
  ['mediaTheme', 'media-theme'],
  ['mediaMinWidth', 'media-min-width'],
  ['mediaMaxWidth', 'media-max-width'],
  ['opacity', 'opacity'],
  ['transitionMs', 'transition-ms'],
  ['transitionProp', 'transition-prop'],
];

const SUPPORTED_PSEUDOS = new Set(['hover', 'focus', 'focus-visible', 'active', 'disabled']);
const SUPPORTED_TRANSITION_PROPS = new Set([
  'all',
  'background',
  'color',
  'opacity',
  'padding',
  'border-radius',
]);

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

function createEmptyParsedRule(
  className: string,
  selectorMeta?: { pseudoClass: string | null; media: MediaContext },
): ParsedStyleRule {
  return {
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
    pseudoClass: selectorMeta?.pseudoClass ?? null,
    mediaTheme: selectorMeta?.media.mediaTheme ?? null,
    mediaMinWidth: selectorMeta?.media.mediaMinWidth ?? null,
    mediaMaxWidth: selectorMeta?.media.mediaMaxWidth ?? null,
    opacity: null,
    transitionMs: null,
    transitionProp: null,
  };
}

function buildRuleKey(rule: ParsedStyleRule): string {
  return [
    rule.className,
    rule.pseudoClass ?? '',
    rule.mediaTheme ?? '',
    rule.mediaMinWidth ?? '',
    rule.mediaMaxWidth ?? '',
  ].join('::');
}

function parseMediaQuery(params: string, inherited: MediaContext): MediaContext {
  const next: MediaContext = {
    mediaTheme: inherited.mediaTheme,
    mediaMinWidth: inherited.mediaMinWidth,
    mediaMaxWidth: inherited.mediaMaxWidth,
    unsupported: inherited.unsupported,
  };
  let recognized = false;
  const lower = params.toLowerCase();

  const themeMatch = lower.match(/prefers-color-scheme\s*:\s*(light|dark)/);
  if (themeMatch) {
    const requestedTheme = themeMatch[1] as SupportedMediaTheme;
    if (next.mediaTheme && next.mediaTheme !== requestedTheme) {
      next.unsupported = true;
    } else {
      next.mediaTheme = requestedTheme;
    }
    recognized = true;
  }

  const minMatch = lower.match(/min-width\s*:\s*(\d+)px/);
  if (minMatch) {
    const minPx = Number.parseInt(minMatch[1], 10);
    if (Number.isFinite(minPx)) {
      next.mediaMinWidth =
        next.mediaMinWidth === null ? minPx : Math.max(next.mediaMinWidth, minPx);
      recognized = true;
    }
  }

  const maxMatch = lower.match(/max-width\s*:\s*(\d+)px/);
  if (maxMatch) {
    const maxPx = Number.parseInt(maxMatch[1], 10);
    if (Number.isFinite(maxPx)) {
      next.mediaMaxWidth =
        next.mediaMaxWidth === null ? maxPx : Math.min(next.mediaMaxWidth, maxPx);
      recognized = true;
    }
  }

  if (!recognized) {
    next.unsupported = true;
  }
  return next;
}

function extractCssBlocks(
  source: string,
  inheritedMedia: MediaContext = { mediaTheme: null, mediaMinWidth: null, mediaMaxWidth: null },
): ExtractedCssBlock[] {
  const blocks: ExtractedCssBlock[] = [];
  let i = 0;
  const len = source.length;

  while (i < len) {
    while (
      i < len &&
      (source[i] === ' ' ||
        source[i] === '\n' ||
        source[i] === '\r' ||
        source[i] === '\t' ||
        source[i] === ';')
    ) {
      i += 1;
    }
    if (i >= len) {
      break;
    }

    const openIdx = source.indexOf('{', i);
    if (openIdx === -1) {
      break;
    }

    const header = source.slice(i, openIdx).trim();
    let depth = 1;
    let cursor = openIdx + 1;
    while (cursor < len && depth > 0) {
      if (source[cursor] === '{') depth += 1;
      else if (source[cursor] === '}') depth -= 1;
      cursor += 1;
    }
    if (depth !== 0) {
      break;
    }

    const body = source.slice(openIdx + 1, cursor - 1);
    i = cursor;

    if (!header) {
      continue;
    }

    if (header.startsWith('@')) {
      const mediaMatch = header.match(/^@media\b([\s\S]*)$/i);
      if (mediaMatch) {
        const nextMedia = parseMediaQuery(mediaMatch[1], inheritedMedia);
        if (!nextMedia.unsupported) {
          blocks.push(...extractCssBlocks(body, nextMedia));
        }
      }
      continue;
    }

    if (!inheritedMedia.unsupported) {
      blocks.push({ selectorText: header, body, media: inheritedMedia });
    }
  }

  return blocks;
}

function isVariableRootSelector(selector: string): boolean {
  const trimmed = selector.trim();
  if (/^(?::root|html|body|:host)$/i.test(trimmed)) {
    return true;
  }
  return /^(?::host)?\[_nghost-[a-zA-Z0-9_-]+(?:=[^\]]*)?\]$/i.test(trimmed);
}

function parseScopedSelectorTarget(rawSelector: string): ParsedSelectorTarget | null {
  let trimmed = rawSelector.trim();
  if (!trimmed) {
    return null;
  }

  let scopeToken: string | null = null;
  const ngMatch = trimmed.match(/\[_ng(?:content|host)-([a-zA-Z0-9_-]+)(?:=[^\]]*)?\]/);
  if (ngMatch) {
    scopeToken = ngMatch[1];
  }
  trimmed = trimmed.replace(/\[_ng(?:content|host)-[^\]]+\]/g, '').trim();

  let pseudoClass: string | null = null;
  const pseudoMatch = trimmed.match(/^(.*?):([a-zA-Z-]+)$/);
  if (pseudoMatch) {
    const candidate = pseudoMatch[2].toLowerCase();
    if (!SUPPORTED_PSEUDOS.has(candidate)) {
      return null;
    }
    pseudoClass = candidate === 'focus-visible' ? 'focus' : candidate;
    trimmed = pseudoMatch[1].trim();
  }

  if ((trimmed === '' || trimmed === ':host') && scopeToken) {
    return {
      baseClass: '__host',
      scopeToken,
      className: `__host__${scopeToken}`,
      pseudoClass,
    };
  }

  const clsMatch = trimmed.match(/^\.([a-zA-Z0-9_-]+)$/);
  if (!clsMatch) {
    return null;
  }
  const baseClass = clsMatch[1];
  return {
    baseClass,
    scopeToken,
    className: scopeToken ? `${baseClass}__${scopeToken}` : baseClass,
    pseudoClass,
  };
}

function splitTopLevelDeclarations(body: string): Array<[string, string]> {
  const decls: Array<[string, string]> = [];
  for (const rawDecl of body.split(';')) {
    const colon = rawDecl.indexOf(':');
    if (colon === -1) {
      continue;
    }
    const prop = rawDecl.slice(0, colon).trim();
    const val = rawDecl.slice(colon + 1).trim();
    if (prop && val) {
      decls.push([prop, val]);
    }
  }
  return decls;
}

function findMatchingParen(input: string, openParenIdx: number): number {
  let depth = 1;
  for (let idx = openParenIdx + 1; idx < input.length; idx += 1) {
    if (input[idx] === '(') depth += 1;
    else if (input[idx] === ')') {
      depth -= 1;
      if (depth === 0) return idx;
    }
  }
  return -1;
}

function splitFirstTopLevelComma(inner: string): [string, string | null] {
  let depth = 0;
  for (let idx = 0; idx < inner.length; idx += 1) {
    const ch = inner[idx];
    if (ch === '(') depth += 1;
    else if (ch === ')') depth -= 1;
    else if (ch === ',' && depth === 0) {
      return [inner.slice(0, idx).trim(), inner.slice(idx + 1).trim()];
    }
  }
  return [inner.trim(), null];
}

function resolveCssVariables(
  rawValue: string,
  vars: ReadonlyMap<string, string>,
  depth = 0,
): string {
  if (depth >= 8 || !rawValue.includes('var(')) {
    return rawValue;
  }
  let out = '';
  let cursor = 0;
  while (cursor < rawValue.length) {
    const varIdx = rawValue.indexOf('var(', cursor);
    if (varIdx === -1) {
      out += rawValue.slice(cursor);
      break;
    }
    out += rawValue.slice(cursor, varIdx);
    const openIdx = varIdx + 3;
    const closeIdx = findMatchingParen(rawValue, openIdx);
    if (closeIdx === -1) {
      out += rawValue.slice(varIdx);
      break;
    }
    const inner = rawValue.slice(openIdx + 1, closeIdx);
    const [varName, fallback] = splitFirstTopLevelComma(inner);
    if (vars.has(varName)) {
      out += resolveCssVariables(vars.get(varName)!, vars, depth + 1);
    } else if (fallback !== null) {
      out += resolveCssVariables(fallback, vars, depth + 1);
    } else {
      out += rawValue.slice(varIdx, closeIdx + 1);
    }
    cursor = closeIdx + 1;
  }
  return out.trim();
}

function parseOpacityPercent(raw: string): number | null {
  const trimmed = raw.trim();
  if (!trimmed) return null;
  if (trimmed.endsWith('%')) {
    const pct = Number.parseFloat(trimmed.slice(0, -1));
    return Number.isFinite(pct) ? Math.max(0, Math.min(100, Math.round(pct))) : null;
  }
  const num = Number.parseFloat(trimmed);
  if (!Number.isFinite(num) || num < 0) return null;
  const pct = num <= 1 ? Math.round(num * 100) : Math.round(num);
  return Math.max(0, Math.min(100, pct));
}

function parseDurationMs(raw: string): number | null {
  const match = raw
    .trim()
    .toLowerCase()
    .match(/^(\d+(?:\.\d+)?)(ms|s)$/);
  if (!match) return null;
  const num = Number.parseFloat(match[1]);
  if (!Number.isFinite(num) || num < 0) return null;
  return match[2] === 's' ? Math.round(num * 1000) : Math.round(num);
}

function mapTransitionProp(token: string): string | null {
  const lower = token.trim().toLowerCase();
  const normalized = lower === 'background-color' ? 'background' : lower;
  return SUPPORTED_TRANSITION_PROPS.has(normalized) ? normalized : null;
}

function applyTransitionDeclaration(rule: ParsedStyleRule, prop: string, val: string): void {
  const firstSegment = val.split(',')[0]?.trim() ?? '';
  if (!firstSegment || firstSegment.toLowerCase() === 'none') {
    return;
  }
  if (prop === 'transition-duration') {
    const ms = parseDurationMs(firstSegment);
    if (ms !== null) rule.transitionMs = ms;
    return;
  }
  if (prop === 'transition-property') {
    const mapped = mapTransitionProp(firstSegment);
    if (mapped) rule.transitionProp = mapped;
    return;
  }
  for (const token of firstSegment.split(/\s+/)) {
    const ms = parseDurationMs(token);
    if (ms !== null) {
      rule.transitionMs = ms;
      continue;
    }
    const mappedProp = mapTransitionProp(token);
    if (mappedProp) {
      rule.transitionProp = mappedProp;
    }
  }
  if (rule.transitionMs !== null && rule.transitionProp === null) {
    rule.transitionProp = 'all';
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

function applyDeclaration(rule: ParsedStyleRule, rawProp: string, val: string): void {
  const prop = rawProp.toLowerCase();
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
  } else if (prop === 'opacity') {
    const op = parseOpacityPercent(val);
    if (op !== null) rule.opacity = op;
  } else if (
    prop === 'transition' ||
    prop === 'transition-duration' ||
    prop === 'transition-property'
  ) {
    applyTransitionDeclaration(rule, prop, val);
  } else {
    applyRuntimeLayoutDeclaration(rule, prop, val);
  }
}

export interface SharedRuntimeCssVars {
  baseVars: Map<string, string>;
  lightVars: Map<string, string>;
  darkVars: Map<string, string>;
}

function parseRuntimeStyleEntries(
  cssSource: string,
  sharedVars?: SharedRuntimeCssVars,
): ParsedStyleEntry[] {
  const cleaned = cssSource.replace(/\/\*[\s\S]*?\*\//g, '');
  const blocks = extractCssBlocks(cleaned);

  const baseVars = sharedVars?.baseVars ?? new Map<string, string>();
  const lightVars = sharedVars?.lightVars ?? new Map<string, string>();
  const darkVars = sharedVars?.darkVars ?? new Map<string, string>();

  for (const block of blocks) {
    const selectors = block.selectorText.split(',');
    if (!selectors.some(isVariableRootSelector)) {
      continue;
    }
    const targetMap =
      block.media.mediaTheme === 'light'
        ? lightVars
        : block.media.mediaTheme === 'dark'
          ? darkVars
          : baseVars;
    for (const [prop, val] of splitTopLevelDeclarations(block.body)) {
      if (prop.startsWith('--')) {
        targetMap.set(prop, val);
      }
    }
  }

  const entriesByKey = new Map<string, ParsedStyleEntry>();

  const upsertRule = (target: ParsedSelectorTarget, media: MediaContext): ParsedStyleRule => {
    const probeRule = createEmptyParsedRule(target.className, {
      pseudoClass: target.pseudoClass,
      media,
    });
    const key = buildRuleKey(probeRule);
    const existing = entriesByKey.get(key);
    if (existing) {
      return existing.rule;
    }
    entriesByKey.set(key, {
      rule: probeRule,
      baseClass: target.baseClass,
      scopeToken: target.scopeToken,
    });
    return probeRule;
  };

  for (const block of blocks) {
    const rawDecls = splitTopLevelDeclarations(block.body);
    const localVars = new Map<string, string>();
    const propDecls: Array<[string, string]> = [];
    for (const [prop, val] of rawDecls) {
      if (prop.startsWith('--')) {
        localVars.set(prop, val);
      } else {
        propDecls.push([prop, val]);
      }
    }
    if (propDecls.length === 0) {
      continue;
    }

    const activeThemeVars =
      block.media.mediaTheme === 'light'
        ? lightVars
        : block.media.mediaTheme === 'dark'
          ? darkVars
          : null;
    const effectiveVars = new Map<string, string>(baseVars);
    if (activeThemeVars) {
      for (const [k, v] of activeThemeVars) effectiveVars.set(k, v);
    }
    for (const [k, v] of localVars) effectiveVars.set(k, v);

    const targets: ParsedSelectorTarget[] = [];
    for (const rawSelector of block.selectorText.split(',')) {
      const target = parseScopedSelectorTarget(rawSelector);
      if (target) targets.push(target);
    }
    if (targets.length === 0) {
      continue;
    }

    for (const target of targets) {
      const rule = upsertRule(target, block.media);
      for (const [prop, rawVal] of propDecls) {
        applyDeclaration(rule, prop, resolveCssVariables(rawVal, effectiveVars));
      }

      if (block.media.mediaTheme === null) {
        for (const [themeName, overrideMap] of [
          ['light', lightVars] as const,
          ['dark', darkVars] as const,
        ]) {
          if (overrideMap.size === 0) {
            continue;
          }
          const themedVars = new Map<string, string>(effectiveVars);
          for (const [k, v] of overrideMap) themedVars.set(k, v);
          for (const [k, v] of localVars) themedVars.set(k, v);

          const changedDecls: Array<[string, string]> = [];
          for (const [prop, rawVal] of propDecls) {
            if (!rawVal.includes('var(')) {
              continue;
            }
            const baseResolved = resolveCssVariables(rawVal, effectiveVars);
            const themedResolved = resolveCssVariables(rawVal, themedVars);
            if (themedResolved !== baseResolved) {
              changedDecls.push([prop, themedResolved]);
            }
          }
          if (changedDecls.length > 0) {
            const themeRule = upsertRule(target, {
              ...block.media,
              mediaTheme: themeName,
            });
            for (const [prop, resolvedVal] of changedDecls) {
              applyDeclaration(themeRule, prop, resolvedVal);
            }
          }
        }
      }
    }
  }

  return [...entriesByKey.values()];
}

/**
 * Parses component CSS rules at runtime into `ParsedStyleRule` descriptors compatible with
 * the Rust engine's CSS Flexbox/Grid, pseudo-classes, `@media`, variables, and encapsulation syntax.
 */
export function parseRuntimeStyles(cssSource: string): ParsedStyleRule[] {
  return parseRuntimeStyleEntries(cssSource).map((entry) => entry.rule);
}

/**
 * Formats optional `ParsedStyleRule` layout/typography/media/pseudo fields as space-separated `key=value` tokens.
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

export function registerParsedStyles(
  bridge: NgNativeBridge,
  styles: readonly string[],
  scopedClassesByScope?: Map<string, Set<string>>,
  sharedVars?: SharedRuntimeCssVars,
): void {
  for (const css of styles) {
    if (typeof css !== 'string' || css.trim().length === 0) {
      continue;
    }
    for (const entry of parseRuntimeStyleEntries(css, sharedVars)) {
      if (entry.scopeToken && scopedClassesByScope) {
        let classes = scopedClassesByScope.get(entry.scopeToken);
        if (!classes) {
          classes = new Set<string>();
          scopedClassesByScope.set(entry.scopeToken, classes);
        }
        classes.add(entry.baseClass);
      }
      bridge.registerStyle(
        entry.rule.className,
        entry.rule.background,
        entry.rule.color,
        entry.rule.padding,
        entry.rule.spacing,
        formatParsedStyleExtensions(entry.rule),
      );
    }
  }
}
