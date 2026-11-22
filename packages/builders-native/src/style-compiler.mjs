import postcss from 'postcss';

const SIMPLE_CLASS_SELECTOR = /^\.([a-zA-Z0-9_-]+)$/;

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

function createEmptyRule(className) {
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
  };
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

function applyDeclaration(rule, prop, value) {
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
    default:
      applyLayoutDeclaration(rule, prop, value);
      break;
  }
}

/**
 * Parses component CSS with the `postcss` AST into the native Style IR.
 * Only simple class selectors (`.name`, including selector lists) are mapped;
 * pseudo-classes, combinators, and at-rules are skipped because the native
 * backends have no equivalent for them yet.
 */
export function parseStylesCss(cssSource, from = 'app.css') {
  const root = postcss.parse(cssSource, { from });
  const rulesByClass = new Map();

  root.walkRules((cssRule) => {
    if (cssRule.parent?.type === 'atrule') return;

    for (const selector of cssRule.selectors) {
      const match = selector.trim().match(SIMPLE_CLASS_SELECTOR);
      if (!match) continue;

      const className = match[1];
      const rule = rulesByClass.get(className) ?? createEmptyRule(className);
      cssRule.walkDecls((decl) => applyDeclaration(rule, decl.prop, decl.value.trim()));
      rulesByClass.set(className, rule);
    }
  });

  return [...rulesByClass.values()];
}

/**
 * Serializes a Style IR rule into a `.ngb` `style` instruction, appending optional
 * `key=value` extensions only when declared (keeps legacy bundles byte-identical).
 */
export function emitStyleInstruction(rule) {
  const base = `style ${rule.className} ${rule.background} ${rule.color} ${rule.padding} ${rule.spacing}`;
  const extensions = [];
  if (rule.borderRadius !== null) extensions.push(`radius=${rule.borderRadius}`);
  if (rule.fontSize !== null) extensions.push(`font-size=${rule.fontSize}`);
  if (rule.fontWeight !== null) extensions.push(`font-weight=${rule.fontWeight}`);
  if (rule.minWidth !== null) extensions.push(`min-width=${rule.minWidth}`);
  if (rule.minHeight !== null) extensions.push(`min-height=${rule.minHeight}`);
  if (rule.orientation !== null) extensions.push(`orientation=${rule.orientation}`);
  if (rule.hexpand !== null) extensions.push(`hexpand=${rule.hexpand}`);
  if (rule.vexpand !== null) extensions.push(`vexpand=${rule.vexpand}`);
  if (rule.halign !== null) extensions.push(`halign=${rule.halign}`);
  if (rule.valign !== null) extensions.push(`valign=${rule.valign}`);
  if (rule.minChildren !== null) extensions.push(`min-children=${rule.minChildren}`);
  if (rule.maxChildren !== null) extensions.push(`max-children=${rule.maxChildren}`);
  if (rule.homogeneous !== null) extensions.push(`homogeneous=${rule.homogeneous}`);
  return extensions.length > 0 ? `${base} ${extensions.join(' ')}` : base;
}
