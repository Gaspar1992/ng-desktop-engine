import postcss from 'postcss';

const SIMPLE_CLASS_SELECTOR = /^\.([a-zA-Z0-9_-]+)$/;
const SUPPORTED_PSEUDOS = new Set(['hover', 'focus', 'focus-visible', 'active', 'disabled']);
const SUPPORTED_TRANSITION_PROPS = new Set([
  'all',
  'background',
  'color',
  'opacity',
  'padding',
  'border-radius',
]);

function toPixelInt(raw) {
  const parsed = Number.parseInt(String(raw).trim().replace(/px$/, ''), 10);
  return Number.isFinite(parsed) && parsed >= 0 ? parsed : null;
}

function mapAlignKeyword(raw) {
  switch (String(raw).trim().toLowerCase()) {
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

function createEmptyRule(className, meta = {}) {
  return {
    className,
    background: 'none',
    color: 'none',
    padding: '0',
    spacing: '0',
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
    pseudoClass: meta.pseudoClass ?? null,
    mediaTheme: meta.mediaTheme ?? null,
    mediaMinWidth: meta.mediaMinWidth ?? null,
    mediaMaxWidth: meta.mediaMaxWidth ?? null,
    opacity: null,
    transitionMs: null,
    transitionProp: null,
  };
}

export function buildStyleRuleKey(rule) {
  return [
    rule.className,
    rule.pseudoClass ?? '',
    rule.mediaTheme ?? '',
    rule.mediaMinWidth ?? '',
    rule.mediaMaxWidth ?? '',
  ].join('::');
}

function parseMediaQuery(params, inherited) {
  const next = {
    mediaTheme: inherited.mediaTheme,
    mediaMinWidth: inherited.mediaMinWidth,
    mediaMaxWidth: inherited.mediaMaxWidth,
    unsupported: inherited.unsupported,
  };
  let recognized = false;
  const lower = String(params).toLowerCase();

  const themeMatch = lower.match(/prefers-color-scheme\s*:\s*(light|dark)/);
  if (themeMatch) {
    const requestedTheme = themeMatch[1];
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

function resolveInheritedMedia(cssRule) {
  const chain = [];
  let cur = cssRule.parent;
  while (cur && cur.type !== 'root') {
    if (cur.type !== 'atrule') {
      return { mediaTheme: null, mediaMinWidth: null, mediaMaxWidth: null, unsupported: true };
    }
    if (String(cur.name).toLowerCase() !== 'media') {
      return { mediaTheme: null, mediaMinWidth: null, mediaMaxWidth: null, unsupported: true };
    }
    chain.unshift(cur.params);
    cur = cur.parent;
  }
  let ctx = { mediaTheme: null, mediaMinWidth: null, mediaMaxWidth: null, unsupported: false };
  for (const params of chain) {
    ctx = parseMediaQuery(params, ctx);
  }
  return ctx;
}

function isVariableRootSelector(selector) {
  const trimmed = selector.trim();
  if (/^(?::root|html|body|:host)$/i.test(trimmed)) {
    return true;
  }
  return /^(?::host)?\[_nghost-[a-zA-Z0-9_-]+(?:=[^\]]*)?\]$/i.test(trimmed);
}

function parseSelectorTarget(rawSelector) {
  let trimmed = String(rawSelector).trim();
  if (!trimmed) return null;

  let scopeToken = null;
  const ngMatch = trimmed.match(/\[_ng(?:content|host)-([a-zA-Z0-9_-]+)(?:=[^\]]*)?\]/);
  if (ngMatch) {
    scopeToken = ngMatch[1];
  }
  trimmed = trimmed.replace(/\[_ng(?:content|host)-[^\]]+\]/g, '').trim();

  let pseudoClass = null;
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
      className: `__host__${scopeToken}`,
      pseudoClass,
    };
  }

  const match = trimmed.match(SIMPLE_CLASS_SELECTOR);
  if (!match) return null;

  const baseClass = match[1];
  return {
    className: scopeToken ? `${baseClass}__${scopeToken}` : baseClass,
    pseudoClass,
  };
}

function findMatchingParen(input, openParenIdx) {
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

function splitFirstTopLevelComma(inner) {
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

function resolveCssVariables(rawValue, vars, depth = 0) {
  const input = String(rawValue);
  if (depth >= 8 || !input.includes('var(')) {
    return input;
  }
  let out = '';
  let cursor = 0;
  while (cursor < input.length) {
    const varIdx = input.indexOf('var(', cursor);
    if (varIdx === -1) {
      out += input.slice(cursor);
      break;
    }
    out += input.slice(cursor, varIdx);
    const openIdx = varIdx + 3;
    const closeIdx = findMatchingParen(input, openIdx);
    if (closeIdx === -1) {
      out += input.slice(varIdx);
      break;
    }
    const inner = input.slice(openIdx + 1, closeIdx);
    const [varName, fallback] = splitFirstTopLevelComma(inner);
    if (vars.has(varName)) {
      out += resolveCssVariables(vars.get(varName), vars, depth + 1);
    } else if (fallback !== null) {
      out += resolveCssVariables(fallback, vars, depth + 1);
    } else {
      out += input.slice(varIdx, closeIdx + 1);
    }
    cursor = closeIdx + 1;
  }
  return out.trim();
}

function parseOpacityPercent(raw) {
  const trimmed = String(raw).trim();
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

function parseDurationMs(raw) {
  const match = String(raw)
    .trim()
    .toLowerCase()
    .match(/^(\d+(?:\.\d+)?)(ms|s)$/);
  if (!match) return null;
  const num = Number.parseFloat(match[1]);
  if (!Number.isFinite(num) || num < 0) return null;
  return match[2] === 's' ? Math.round(num * 1000) : Math.round(num);
}

function mapTransitionProp(token) {
  const lower = String(token).trim().toLowerCase();
  const normalized = lower === 'background-color' ? 'background' : lower;
  return SUPPORTED_TRANSITION_PROPS.has(normalized) ? normalized : null;
}

function applyTransitionDeclaration(rule, prop, val) {
  const firstSegment = String(val).split(',')[0]?.trim() ?? '';
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

function applyGridTemplateColumns(rule, value) {
  const match = String(value)
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
  if (match[2]) {
    rule.minWidth ??= Number.parseInt(match[2], 10);
  }
}

function applyLayoutDeclaration(rule, prop, value) {
  const lower = String(value).trim().toLowerCase();
  switch (prop) {
    case 'flex-direction':
      if (lower.startsWith('row')) rule.orientation = 'horizontal';
      else if (lower.startsWith('column')) rule.orientation = 'vertical';
      break;
    case 'flex-wrap':
      if (lower.startsWith('wrap')) {
        rule.minChildren ??= 1;
        rule.maxChildren ??= 12;
        rule.homogeneous = true;
        rule.hexpand = true;
      }
      break;
    case 'flex':
    case 'flex-grow': {
      const grow = Number.parseFloat(lower);
      if (Number.isFinite(grow) && grow >= 1) rule.hexpand = true;
      if (prop === 'flex') {
        const basisMatch = lower.match(/(\d+)px/);
        if (basisMatch) rule.minWidth ??= Number.parseInt(basisMatch[1], 10);
      }
      break;
    }
    case 'width':
      if (lower === '100%') rule.hexpand = true;
      else {
        const px = toPixelInt(value);
        if (px !== null) rule.minWidth ??= px;
      }
      break;
    case 'height':
      if (lower === '100%') rule.vexpand = true;
      else {
        const px = toPixelInt(value);
        if (px !== null) rule.minHeight ??= px;
      }
      break;
    case 'grid-template-columns':
      applyGridTemplateColumns(rule, value);
      break;
    case 'justify-self': {
      const aligned = mapAlignKeyword(value);
      if (aligned) rule.halign = aligned;
      break;
    }
    case 'align-self': {
      const aligned = mapAlignKeyword(value);
      if (aligned) rule.valign = aligned;
      break;
    }
    case 'justify-content': {
      const aligned = mapAlignKeyword(value);
      if (!rule.hexpand && aligned) rule.halign ??= aligned;
      break;
    }
    case 'align-items': {
      const aligned = mapAlignKeyword(value);
      if (!rule.vexpand && aligned) rule.valign ??= aligned;
      break;
    }
    default:
      break;
  }
}

function applyDeclaration(rule, rawProp, value) {
  const prop = String(rawProp).toLowerCase();
  switch (prop) {
    case 'background':
    case 'background-color':
      rule.background = value;
      break;
    case 'color':
      rule.color = value;
      break;
    case 'padding': {
      const px = toPixelInt(value);
      if (px !== null) rule.padding = String(px);
      break;
    }
    case 'gap':
    case 'spacing': {
      const px = toPixelInt(value);
      if (px !== null) rule.spacing = String(px);
      break;
    }
    case 'row-gap':
    case 'column-gap': {
      if (rule.spacing === '0') {
        const px = toPixelInt(value);
        if (px !== null) rule.spacing = String(px);
      }
      break;
    }
    case 'border-radius':
      rule.borderRadius = toPixelInt(value);
      break;
    case 'font-size':
      rule.fontSize = toPixelInt(value);
      break;
    case 'font-weight': {
      const keyword = { normal: 400, bold: 700 }[value];
      rule.fontWeight = keyword ?? toPixelInt(value);
      break;
    }
    case 'min-width':
      rule.minWidth = toPixelInt(value);
      break;
    case 'min-height':
      rule.minHeight = toPixelInt(value);
      break;
    case 'opacity': {
      const op = parseOpacityPercent(value);
      if (op !== null) rule.opacity = op;
      break;
    }
    case 'transition':
    case 'transition-duration':
    case 'transition-property':
      applyTransitionDeclaration(rule, prop, value);
      break;
    default:
      applyLayoutDeclaration(rule, prop, value);
      break;
  }
}

/**
 * Extracts `--var` custom properties from `:root` / `host` rules and `@media (prefers-color-scheme)` blocks.
 */
export function extractCssVariables(
  cssSource,
  from = 'styles.css',
  target = { baseVars: new Map(), lightVars: new Map(), darkVars: new Map() },
) {
  const baseVars = target?.baseVars ?? new Map();
  const lightVars = target?.lightVars ?? new Map();
  const darkVars = target?.darkVars ?? new Map();
  const root = postcss.parse(cssSource, { from });
  root.walkRules((cssRule) => {
    const media = resolveInheritedMedia(cssRule);
    if (media.unsupported) return;
    if (!cssRule.selectors.some(isVariableRootSelector)) return;
    const targetMap =
      media.mediaTheme === 'light' ? lightVars : media.mediaTheme === 'dark' ? darkVars : baseVars;
    cssRule.walkDecls((decl) => {
      if (decl.prop.startsWith('--')) {
        targetMap.set(decl.prop, decl.value.trim());
      }
    });
  });
  return { baseVars, lightVars, darkVars };
}

/**
 * Parses component CSS with the `postcss` AST into the native Style IR, supporting simple
 * class selectors, interactive pseudo-classes, `@media` queries, `--var`/`var()` tokens
 * with automatic light/dark `@media` variant synthesis, `opacity`, `transition`, and
 * Angular `ViewEncapsulation.Emulated` scoping tokens.
 */
export function parseStylesCss(cssSource, from = 'app.css', sharedVars = null) {
  const root = postcss.parse(cssSource, { from });
  const baseVars = new Map(sharedVars?.baseVars ?? []);
  const lightVars = new Map(sharedVars?.lightVars ?? []);
  const darkVars = new Map(sharedVars?.darkVars ?? []);

  extractCssVariables(cssSource, from, { baseVars, lightVars, darkVars });

  const rulesByKey = new Map();
  const upsertRule = (target, media) => {
    const probe = createEmptyRule(target.className, {
      pseudoClass: target.pseudoClass,
      mediaTheme: media.mediaTheme,
      mediaMinWidth: media.mediaMinWidth,
      mediaMaxWidth: media.mediaMaxWidth,
    });
    const key = buildStyleRuleKey(probe);
    const existing = rulesByKey.get(key);
    if (existing) return existing;
    rulesByKey.set(key, probe);
    return probe;
  };

  root.walkRules((cssRule) => {
    const media = resolveInheritedMedia(cssRule);
    if (media.unsupported) return;

    const localVars = new Map();
    const propDecls = [];
    cssRule.walkDecls((decl) => {
      const val = decl.value.trim();
      if (!val) return;
      if (decl.prop.startsWith('--')) {
        localVars.set(decl.prop, val);
      } else {
        propDecls.push([decl.prop, val]);
      }
    });
    if (propDecls.length === 0) return;

    const activeThemeVars =
      media.mediaTheme === 'light' ? lightVars : media.mediaTheme === 'dark' ? darkVars : null;
    const effectiveVars = new Map(baseVars);
    if (activeThemeVars) {
      for (const [k, v] of activeThemeVars) effectiveVars.set(k, v);
    }
    for (const [k, v] of localVars) effectiveVars.set(k, v);

    for (const selector of cssRule.selectors) {
      const target = parseSelectorTarget(selector);
      if (!target) continue;

      const rule = upsertRule(target, media);
      for (const [prop, rawVal] of propDecls) {
        applyDeclaration(rule, prop, resolveCssVariables(rawVal, effectiveVars));
      }

      if (media.mediaTheme === null) {
        for (const [themeName, overrideMap] of [
          ['light', lightVars],
          ['dark', darkVars],
        ]) {
          if (overrideMap.size === 0) continue;
          const themedVars = new Map(effectiveVars);
          for (const [k, v] of overrideMap) themedVars.set(k, v);
          for (const [k, v] of localVars) themedVars.set(k, v);

          const changedDecls = [];
          for (const [prop, rawVal] of propDecls) {
            if (!rawVal.includes('var(')) continue;
            const baseResolved = resolveCssVariables(rawVal, effectiveVars);
            const themedResolved = resolveCssVariables(rawVal, themedVars);
            if (themedResolved !== baseResolved) {
              changedDecls.push([prop, themedResolved]);
            }
          }
          if (changedDecls.length > 0) {
            const themeRule = upsertRule(target, { ...media, mediaTheme: themeName });
            for (const [prop, resolvedVal] of changedDecls) {
              applyDeclaration(themeRule, prop, resolvedVal);
            }
          }
        }
      }
    }
  });

  return [...rulesByKey.values()];
}

const STYLE_EXTENSION_FIELDS = [
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

/**
 * Formats optional Style IR typography/layout/media/pseudo extensions as space-separated `key=value` tokens.
 */
export function formatStyleRuleExtensions(rule) {
  const extensions = [];
  for (const [prop, key] of STYLE_EXTENSION_FIELDS) {
    const value = rule?.[prop];
    if (value !== null && value !== undefined) {
      extensions.push(`${key}=${value}`);
    }
  }
  return extensions.join(' ');
}

/**
 * Serializes a Style IR rule into a `.ngb` `style` instruction, appending optional
 * `key=value` extensions only when declared (keeps legacy bundles byte-identical).
 */
export function emitStyleInstruction(rule) {
  const base = `style ${rule.className} ${rule.background} ${rule.color} ${rule.padding} ${rule.spacing}`;
  const extras = formatStyleRuleExtensions(rule);
  return extras.length > 0 ? `${base} ${extras}` : base;
}
