import {
  formatParsedStyleExtensions,
  type ParsedStyleRule,
  parseRuntimeStyles,
  resolveNativeTag,
} from '@ng-desktop/platform';

export interface PlaygroundAttribute {
  readonly name: string;
  readonly value: string;
}

export interface PlaygroundPreviewStyle {
  readonly background?: string;
  readonly color?: string;
  readonly paddingPx?: number;
  readonly gapPx?: number;
  readonly borderRadiusPx?: number;
  readonly fontSizePx?: number;
  readonly fontWeight?: number;
  readonly minWidthPx?: number;
  readonly minHeightPx?: number;
  readonly orientation?: 'horizontal' | 'vertical';
  readonly opacity?: number;
}

export interface PlaygroundPreviewNode {
  readonly id: number;
  readonly sourceTag: string;
  readonly nativeTag: string;
  readonly attributes: readonly PlaygroundAttribute[];
  readonly events: readonly string[];
  readonly classes: readonly string[];
  readonly style: PlaygroundPreviewStyle;
  readonly textContent: string;
  readonly children: readonly PlaygroundPreviewNode[];
}

export interface NgbInstructionLine {
  readonly opcode:
    | 'Style'
    | 'CreateNode'
    | 'SetAttribute'
    | 'SetClasses'
    | 'AttachEvent'
    | 'AppendChild';
  readonly summary: string;
}

export interface PlaygroundCompilationResult {
  readonly instructions: readonly NgbInstructionLine[];
  readonly styleRules: readonly ParsedStyleRule[];
  readonly roots: readonly PlaygroundPreviewNode[];
  readonly warnings: readonly string[];
  readonly nodeCount: number;
  readonly styleRuleCount: number;
  readonly instructionCount: number;
}

const HTML_VOID_TAGS = new Set(['br', 'hr', 'img', 'input', 'meta', 'link']);

// Leaf native widgets accept closing tags for Angular template ergonomics (`<ng-label></ng-label>`),
// but never hold nested child widgets (`ng-header-bar` is a container and is intentionally excluded).
const LEAF_NATIVE_TAGS = new Set([
  'ng-separator',
  'ng-entry',
  'ng-password-entry',
  'ng-search-entry',
  'ng-spin-button',
  'ng-switch',
  'ng-checkbox',
  'ng-check',
  'gtk-check-button',
  'ng-slider',
  'ng-scale',
  'ng-select',
  'ng-dropdown',
  'gtk-drop-down',
  'ng-level-bar',
  'ng-spinner',
  'ng-progress-bar',
  'ng-label',
  'ng-button',
  'ng-image',
]);

interface MutableParsedNode {
  id: number;
  sourceTag: string;
  nativeTag: string;
  attributes: PlaygroundAttribute[];
  events: string[];
  classes: string[];
  textParts: string[];
  children: MutableParsedNode[];
}

type TemplateToken =
  | { kind: 'text'; value: string }
  | {
      kind: 'tag';
      isClosing: boolean;
      rawTag: string;
      rawAttrs: string;
      selfClosing: boolean;
    };

const ATTR_REGEX = /([^\s=/>"'`]+)(?:\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s"'=<>`]+)))?/g;

function scanTemplateTokens(template: string): TemplateToken[] {
  const tokens: TemplateToken[] = [];
  let cursor = 0;
  let textStart = 0;
  const len = template.length;

  const flushText = (end: number): void => {
    if (end > textStart) {
      tokens.push({ kind: 'text', value: template.slice(textStart, end) });
    }
  };

  while (cursor < len) {
    const ltIndex = template.indexOf('<', cursor);
    if (ltIndex === -1) {
      break;
    }

    if (template.startsWith('<!--', ltIndex)) {
      flushText(ltIndex);
      const commentEnd = template.indexOf('-->', ltIndex + 4);
      cursor = commentEnd === -1 ? len : commentEnd + 3;
      textStart = cursor;
      continue;
    }

    const afterLt = ltIndex + 1;
    const isClosing = template[afterLt] === '/';
    const nameStart = isClosing ? afterLt + 1 : afterLt;
    if (nameStart >= len || !/[a-zA-Z]/.test(template[nameStart] ?? '')) {
      cursor = ltIndex + 1;
      continue;
    }

    let nameEnd = nameStart;
    while (nameEnd < len && /[a-zA-Z0-9_-]/.test(template[nameEnd] ?? '')) {
      nameEnd += 1;
    }

    // Scan until an unquoted `>` so arrow functions like `(click)="n => n + 1"` stay intact.
    let attrPos = nameEnd;
    let quote: '"' | "'" | null = null;
    while (attrPos < len) {
      const ch = template[attrPos] ?? '';
      if (quote) {
        if (ch === '\\' && attrPos + 1 < len) {
          attrPos += 2;
          continue;
        }
        if (ch === quote) {
          quote = null;
        }
      } else {
        if (ch === '"' || ch === "'") {
          quote = ch;
        } else if (ch === '>') {
          break;
        }
      }
      attrPos += 1;
    }

    if (attrPos >= len) {
      cursor = ltIndex + 1;
      continue;
    }

    flushText(ltIndex);
    const rawTag = template.slice(nameStart, nameEnd).toLowerCase();
    let rawAttrs = template.slice(nameEnd, attrPos);
    let selfClosing = false;
    const trimmedRight = rawAttrs.trimEnd();
    if (!isClosing && trimmedRight.endsWith('/')) {
      selfClosing = true;
      rawAttrs = trimmedRight.slice(0, -1);
    }

    tokens.push({
      kind: 'tag',
      isClosing,
      rawTag,
      rawAttrs,
      selfClosing,
    });

    cursor = attrPos + 1;
    textStart = cursor;
  }

  flushText(len);
  return tokens;
}

function normalizeAttrName(raw: string): {
  kind: 'attr' | 'event' | 'class';
  name: string;
} {
  if (raw.startsWith('(') && raw.endsWith(')')) {
    return { kind: 'event', name: raw.slice(1, -1) };
  }
  if (raw.startsWith('[') && raw.endsWith(']')) {
    const inner = raw.slice(1, -1);
    return inner === 'class' ? { kind: 'class', name: 'class' } : { kind: 'attr', name: inner };
  }
  if (raw === 'class') {
    return { kind: 'class', name: 'class' };
  }
  return { kind: 'attr', name: raw };
}

function parseAttributes(rawAttrs: string): {
  attributes: PlaygroundAttribute[];
  events: string[];
  classes: string[];
} {
  const attributes: PlaygroundAttribute[] = [];
  const events: string[] = [];
  const classes: string[] = [];

  for (const match of rawAttrs.matchAll(ATTR_REGEX)) {
    const rawName = match[1];
    if (!rawName) {
      continue;
    }
    const rawValue = match[2] ?? match[3] ?? match[4] ?? 'true';
    const normalized = normalizeAttrName(rawName);
    if (normalized.kind === 'event') {
      events.push(normalized.name);
    } else if (normalized.kind === 'class') {
      for (const token of rawValue.split(/\s+/)) {
        const trimmed = token.trim();
        if (trimmed) {
          classes.push(trimmed);
        }
      }
    } else {
      attributes.push({ name: normalized.name, value: rawValue });
    }
  }

  return { attributes, events, classes };
}

function resolvePreviewStyle(
  classes: readonly string[],
  rules: readonly ParsedStyleRule[],
  theme: 'light' | 'dark',
): PlaygroundPreviewStyle {
  if (classes.length === 0) {
    return {};
  }
  const classSet = new Set(classes);
  let background: string | undefined;
  let color: string | undefined;
  let paddingPx: number | undefined;
  let gapPx: number | undefined;
  let borderRadiusPx: number | undefined;
  let fontSizePx: number | undefined;
  let fontWeight: number | undefined;
  let minWidthPx: number | undefined;
  let minHeightPx: number | undefined;
  let orientation: 'horizontal' | 'vertical' | undefined;
  let opacity: number | undefined;

  const applyRule = (rule: ParsedStyleRule): void => {
    if (rule.background && rule.background !== 'none') background = rule.background;
    if (rule.color && rule.color !== 'none') color = rule.color;
    if (rule.padding > 0) paddingPx = rule.padding;
    if (rule.spacing > 0) gapPx = rule.spacing;
    if (rule.borderRadius !== null) borderRadiusPx = rule.borderRadius;
    if (rule.fontSize !== null) fontSizePx = rule.fontSize;
    if (rule.fontWeight !== null) fontWeight = rule.fontWeight;
    if (rule.minWidth !== null) minWidthPx = rule.minWidth;
    if (rule.minHeight !== null) minHeightPx = rule.minHeight;
    if (rule.orientation !== null) orientation = rule.orientation;
    if (rule.opacity !== null) opacity = rule.opacity;
  };

  for (const rule of rules) {
    if (classSet.has(rule.className) && rule.pseudoClass === null && rule.mediaTheme === null) {
      applyRule(rule);
    }
  }
  for (const rule of rules) {
    if (classSet.has(rule.className) && rule.pseudoClass === null && rule.mediaTheme === theme) {
      applyRule(rule);
    }
  }

  return {
    ...(background !== undefined ? { background } : {}),
    ...(color !== undefined ? { color } : {}),
    ...(paddingPx !== undefined ? { paddingPx } : {}),
    ...(gapPx !== undefined ? { gapPx } : {}),
    ...(borderRadiusPx !== undefined ? { borderRadiusPx } : {}),
    ...(fontSizePx !== undefined ? { fontSizePx } : {}),
    ...(fontWeight !== undefined ? { fontWeight } : {}),
    ...(minWidthPx !== undefined ? { minWidthPx } : {}),
    ...(minHeightPx !== undefined ? { minHeightPx } : {}),
    ...(orientation !== undefined ? { orientation } : {}),
    ...(opacity !== undefined ? { opacity } : {}),
  };
}

function toPreviewNode(
  node: MutableParsedNode,
  rules: readonly ParsedStyleRule[],
  theme: 'light' | 'dark',
): PlaygroundPreviewNode {
  return {
    id: node.id,
    sourceTag: node.sourceTag,
    nativeTag: node.nativeTag,
    attributes: node.attributes,
    events: node.events,
    classes: node.classes,
    style: resolvePreviewStyle(node.classes, rules, theme),
    textContent: node.textParts.join(' ').replace(/\s+/g, ' ').trim(),
    children: node.children.map((child) => toPreviewNode(child, rules, theme)),
  };
}

/**
 * Compiles an Angular/HTML snippet and stylesheet into `.ngb`-equivalent instructions
 * and a themed native widget preview tree in pure browser memory using `@ng-desktop/platform`.
 */
export function compilePlaygroundToNgb(
  template: string,
  css: string,
  theme: 'light' | 'dark',
): PlaygroundCompilationResult {
  const warnings: string[] = [];
  let styleRules: ParsedStyleRule[] = [];

  try {
    styleRules = parseRuntimeStyles(css);
  } catch (err) {
    warnings.push(`CSS parser error: ${err instanceof Error ? err.message : String(err)}`);
  }

  const instructions: NgbInstructionLine[] = [];
  for (const rule of styleRules) {
    const ext = formatParsedStyleExtensions(rule);
    const details = [
      rule.background ? `bg=${rule.background}` : null,
      rule.color ? `color=${rule.color}` : null,
      rule.padding > 0 ? `pad=${rule.padding}` : null,
      rule.spacing > 0 ? `gap=${rule.spacing}` : null,
      ext || null,
    ]
      .filter(Boolean)
      .join(' ');
    const qualifiers = [
      rule.pseudoClass ? `:${rule.pseudoClass}` : '',
      rule.mediaTheme ? ` @media(${rule.mediaTheme})` : '',
    ].join('');
    instructions.push({
      opcode: 'Style',
      summary: `.${rule.className}${qualifiers}${details ? ` { ${details} }` : ''}`,
    });
  }

  const roots: MutableParsedNode[] = [];
  const stack: MutableParsedNode[] = [];
  let nextId = 1;

  const pushText = (raw: string): void => {
    const cleaned = raw.trim();
    if (!cleaned || cleaned.startsWith('@if') || cleaned.startsWith('@for') || cleaned === '}') {
      return;
    }
    const current = stack.at(-1);
    if (current) {
      current.textParts.push(cleaned);
    }
  };

  for (const token of scanTemplateTokens(template)) {
    if (token.kind === 'text') {
      pushText(token.value);
      continue;
    }

    const { isClosing, rawTag, rawAttrs, selfClosing } = token;

    if (isClosing) {
      for (let idx = stack.length - 1; idx >= 0; idx -= 1) {
        if (stack[idx]?.sourceTag === rawTag) {
          stack.length = idx;
          break;
        }
      }
      continue;
    }

    const resolvedTag = resolveNativeTag(rawTag);
    if (!resolvedTag) {
      if (rawTag === 'script' || rawTag === 'style') {
        warnings.push(
          `Skipped <${rawTag}>: native desktop bundles do not embed WebView scripting or DOM style tags.`,
        );
      }
      continue;
    }

    // Pop an unclosed leaf native widget if another element starts after it so siblings do not nest inside leaves.
    while (stack.length > 0 && LEAF_NATIVE_TAGS.has(stack.at(-1)!.nativeTag)) {
      stack.pop();
    }

    const { attributes, events, classes } = parseAttributes(rawAttrs);
    const node: MutableParsedNode = {
      id: nextId++,
      sourceTag: rawTag,
      nativeTag: resolvedTag,
      attributes,
      events,
      classes,
      textParts: [],
      children: [],
    };

    const parent = stack.at(-1);
    if (parent) {
      parent.children.push(node);
    } else {
      roots.push(node);
    }

    instructions.push({
      opcode: 'CreateNode',
      summary: `#${node.id} <${node.nativeTag}> (from <${node.sourceTag}>)`,
    });
    for (const attr of attributes) {
      instructions.push({
        opcode: 'SetAttribute',
        summary: `#${node.id} ${attr.name}="${attr.value}"`,
      });
    }
    if (classes.length > 0) {
      instructions.push({
        opcode: 'SetClasses',
        summary: `#${node.id} [${classes.join(', ')}]`,
      });
    }
    for (const ev of events) {
      instructions.push({
        opcode: 'AttachEvent',
        summary: `#${node.id} on(${ev})`,
      });
    }
    if (parent) {
      instructions.push({
        opcode: 'AppendChild',
        summary: `#${parent.id} <- #${node.id}`,
      });
    }

    if (!selfClosing && !HTML_VOID_TAGS.has(rawTag)) {
      stack.push(node);
    }
  }

  if (roots.length === 0 && template.trim().length > 0) {
    warnings.push(
      'No native widget primitives detected. Try <ng-window>, <ng-box>, <ng-label>, <ng-button>, or standard HTML tags (<div>, <p>, <button>).',
    );
  }

  const nodeCount = nextId - 1;
  return {
    instructions,
    styleRules,
    roots: roots.map((r) => toPreviewNode(r, styleRules, theme)),
    warnings,
    nodeCount,
    styleRuleCount: styleRules.length,
    instructionCount: instructions.length,
  };
}
