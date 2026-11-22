import { ParenthesizedExpression, parseTemplate, PrefixNot } from '@angular/compiler';

const CURRENCY_SYMBOLS = {
  USD: '$',
  EUR: '€',
  GBP: '£',
  JPY: '¥',
};

function parseDecimalDigitsInfo(digitsInfo = '1.0-3') {
  const match = String(digitsInfo).match(/^(\d+)\.(\d+)-(\d+)$/);
  if (!match) {
    return { minInt: 1, minFrac: 0, maxFrac: 3 };
  }
  const minInt = Number.parseInt(match[1], 10);
  const minFrac = Number.parseInt(match[2], 10);
  const maxFrac = Math.max(minFrac, Number.parseInt(match[3], 10));
  return { minInt, minFrac, maxFrac };
}

function formatNumberDigits(rawValue, digitsInfo = '1.0-3') {
  const num = Number(rawValue);
  if (!Number.isFinite(num)) return String(rawValue ?? '');
  const { minInt, minFrac, maxFrac } = parseDecimalDigitsInfo(digitsInfo);
  return new Intl.NumberFormat('en-US', {
    minimumIntegerDigits: Math.max(1, minInt),
    minimumFractionDigits: minFrac,
    maximumFractionDigits: maxFrac,
  }).format(num);
}

/**
 * Applies built-in Angular pipe transformations (`uppercase`, `lowercase`, `titlecase`,
 * `number`, `currency`, `percent`, `slice`, `json`, `default`) at template compile time.
 */
export function applyPipeTransform(value, pipeName, args = []) {
  const name = String(pipeName).trim();
  switch (name) {
    case 'uppercase':
      return String(value ?? '').toUpperCase();
    case 'lowercase':
      return String(value ?? '').toLowerCase();
    case 'titlecase':
      return String(value ?? '').replace(/\w\S*/g, (word) => {
        return word.charAt(0).toUpperCase() + word.slice(1).toLowerCase();
      });
    case 'number':
    case 'decimal':
      return formatNumberDigits(value, args[0] ?? '1.0-3');
    case 'currency': {
      const code = String(args[0] ?? 'USD').toUpperCase();
      const display = String(args[1] ?? 'symbol');
      const symbol = display === 'code' ? `${code} ` : (CURRENCY_SYMBOLS[code] ?? `${code} `);
      const formatted = formatNumberDigits(value, args[2] ?? '1.2-2');
      return `${symbol}${formatted}`;
    }
    case 'percent': {
      const num = Number(value);
      if (!Number.isFinite(num)) return String(value ?? '');
      return `${formatNumberDigits(num * 100, args[0] ?? '1.0-2')}%`;
    }
    case 'slice': {
      const start = Number(args[0] ?? 0);
      const end = args[1] !== undefined ? Number(args[1]) : undefined;
      if (Array.isArray(value) || typeof value === 'string') {
        return value.slice(start, end);
      }
      return String(value ?? '').slice(start, end);
    }
    case 'json':
      return typeof value === 'string' ? value : JSON.stringify(value);
    case 'default':
    case 'coalesce':
      return value === null || value === undefined || value === '' ? (args[0] ?? '') : value;
    default:
      return value;
  }
}

function readStatePath(pathParts, state) {
  if (pathParts.length === 0) return { found: false, value: undefined };

  const joined = pathParts.join('.');
  if (joined in state) {
    return { found: true, value: state[joined] };
  }

  let current = state[pathParts[0]];
  if (!(pathParts[0] in state)) {
    return { found: false, value: undefined };
  }

  for (let i = 1; i < pathParts.length; i++) {
    if (typeof current === 'string' && current.trim().startsWith('{')) {
      try {
        current = JSON.parse(current);
      } catch {
        return { found: false, value: undefined };
      }
    }
    if (current && typeof current === 'object' && pathParts[i] in current) {
      current = current[pathParts[i]];
    } else {
      return { found: false, value: undefined };
    }
  }
  return { found: true, value: current };
}

function collectReceiverChain(exprAst) {
  if (!exprAst) return null;
  if (exprAst.exp && typeof exprAst.name === 'string') {
    return collectReceiverChain(exprAst.exp);
  }
  if (exprAst.receiver && Array.isArray(exprAst.args)) {
    return collectReceiverChain(exprAst.receiver);
  }
  if (typeof exprAst.name === 'string') {
    if (!exprAst.receiver || exprAst.receiver.name === undefined) {
      return [exprAst.name];
    }
    const parent = collectReceiverChain(exprAst.receiver);
    return parent ? [...parent, exprAst.name] : [exprAst.name];
  }
  if (exprAst.receiver && typeof exprAst.receiver.name === 'string') {
    return [exprAst.receiver.name];
  }
  return null;
}

function extractExpressionIdentifier(exprAst) {
  const chain = collectReceiverChain(exprAst);
  return chain && chain.length > 0 ? chain.join('.') : null;
}

function evaluateScalarAst(ast, state) {
  if (!ast) {
    return { value: undefined, signalName: null, pipes: [] };
  }

  if (ast instanceof ParenthesizedExpression) {
    return evaluateScalarAst(ast.expression, state);
  }

  if (ast instanceof PrefixNot) {
    const inner = evaluateScalarAst(ast.expression, state);
    return { value: !isTruthyCondition(inner.value), signalName: null, pipes: [] };
  }

  if (typeof ast.name === 'string' && 'exp' in ast && Array.isArray(ast.args)) {
    const inner = evaluateScalarAst(ast.exp, state);
    const evaluatedArgs = ast.args.map((argAst) => evaluateScalarAst(argAst, state).value);
    const transformed = applyPipeTransform(inner.value, ast.name, evaluatedArgs);
    return {
      value: transformed,
      signalName: inner.signalName,
      pipes: [...inner.pipes, ast.name],
    };
  }

  if ('value' in ast && typeof ast.value !== 'object') {
    return { value: ast.value, signalName: null, pipes: [] };
  }

  if (Array.isArray(ast.elements)) {
    return {
      value: ast.elements.map((el) => evaluateScalarAst(el, state).value),
      signalName: null,
      pipes: [],
    };
  }

  if (typeof ast.operation === 'string' && ast.left !== undefined && ast.right !== undefined) {
    const l = evaluateScalarAst(ast.left, state).value;
    const r = evaluateScalarAst(ast.right, state).value;
    switch (ast.operation) {
      case '+':
        return {
          value:
            typeof l === 'string' || typeof r === 'string' ? `${l}${r}` : Number(l) + Number(r),
          signalName: null,
          pipes: [],
        };
      case '-':
        return { value: Number(l) - Number(r), signalName: null, pipes: [] };
      case '*':
        return { value: Number(l) * Number(r), signalName: null, pipes: [] };
      case '/':
        return { value: Number(r) === 0 ? 0 : Number(l) / Number(r), signalName: null, pipes: [] };
      case '==':
      case '===':
        return { value: String(l) === String(r), signalName: null, pipes: [] };
      case '!=':
      case '!==':
        return { value: String(l) !== String(r), signalName: null, pipes: [] };
      case '>':
        return { value: Number(l) > Number(r), signalName: null, pipes: [] };
      case '<':
        return { value: Number(l) < Number(r), signalName: null, pipes: [] };
      case '>=':
        return { value: Number(l) >= Number(r), signalName: null, pipes: [] };
      case '<=':
        return { value: Number(l) <= Number(r), signalName: null, pipes: [] };
      case '&&':
        return { value: Boolean(l) && Boolean(r), signalName: null, pipes: [] };
      case '||':
        return { value: Boolean(l) || Boolean(r), signalName: null, pipes: [] };
      case '??':
        return { value: l ?? r, signalName: null, pipes: [] };
      default:
        break;
    }
  }

  if (ast.condition !== undefined && ast.trueExp !== undefined && ast.falseExp !== undefined) {
    const cond = evaluateScalarAst(ast.condition, state).value;
    const isTruthy = cond !== 'false' && cond !== '0' && cond !== '' && Boolean(cond);
    return isTruthy
      ? evaluateScalarAst(ast.trueExp, state)
      : evaluateScalarAst(ast.falseExp, state);
  }

  const chain = collectReceiverChain(ast);
  if (chain) {
    const lookedUp = readStatePath(chain, state);
    return {
      value: lookedUp.found ? lookedUp.value : undefined,
      signalName: chain.length === 1 ? chain[0] : null,
      pipes: [],
    };
  }

  return { value: undefined, signalName: null, pipes: [] };
}

function buildPipePlaceholder(pipes) {
  if (!pipes || pipes.length === 0) return '{}';
  const last = pipes[pipes.length - 1];
  if (last === 'uppercase' || last === 'lowercase' || last === 'titlecase' || last === 'percent') {
    return `{|${last}}`;
  }
  return '{}';
}

function evaluateBoundValue(boundValue, state) {
  const ast = boundValue?.ast;
  if (!ast) {
    return { text: '', rawValue: undefined, signalName: null, templatePattern: '{}' };
  }

  if (Array.isArray(ast.strings) && Array.isArray(ast.expressions)) {
    let text = '';
    for (let i = 0; i < ast.strings.length; i++) {
      text += ast.strings[i];
      if (i < ast.expressions.length) {
        const res = evaluateScalarAst(ast.expressions[i], state);
        if (res.value !== undefined && res.value !== null) {
          text += String(res.value);
        }
      }
    }

    let signalName = null;
    let templatePattern = text.trim();
    if (ast.expressions.length === 1) {
      const only = evaluateScalarAst(ast.expressions[0], state);
      signalName = only.signalName;
      const placeholder = buildPipePlaceholder(only.pipes);
      templatePattern = `${ast.strings[0]}${placeholder}${ast.strings[1] ?? ''}`.trim();
    }

    return { text: text.trim(), rawValue: text.trim(), signalName, templatePattern };
  }

  const evaluated = evaluateScalarAst(ast, state);
  const rawValue = evaluated.value;
  const text = rawValue !== undefined && rawValue !== null ? String(rawValue) : '';
  return {
    text,
    rawValue,
    signalName: evaluated.signalName,
    templatePattern: buildPipePlaceholder(evaluated.pipes),
  };
}

function isTruthyCondition(val) {
  return val !== 'false' && val !== '0' && val !== '' && Boolean(val);
}

function isBranchActive(branch, state) {
  if (!branch.expression) {
    return true;
  }
  const evaluated = evaluateScalarAst(branch.expression.ast, state);
  if (evaluated.value === undefined) {
    const ident = extractExpressionIdentifier(branch.expression.ast);
    if (!ident || !(ident in state)) {
      return true;
    }
    return isTruthyCondition(state[ident]);
  }
  return isTruthyCondition(evaluated.value);
}

function normalizeComparisonOp(op) {
  if (op === '===') return '==';
  if (op === '!==') return '!=';
  if (op === '==' || op === '!=' || op === '>' || op === '>=' || op === '<' || op === '<=') {
    return op;
  }
  return null;
}

function invertComparisonOp(op) {
  switch (op) {
    case '==':
      return '!=';
    case '!=':
      return '==';
    case '>':
      return '<=';
    case '<=':
      return '>';
    case '<':
      return '>=';
    case '>=':
      return '<';
    default:
      return '!=';
  }
}

function extractReactiveCondition(ast, numericSignals, signalAliases = {}) {
  if (!ast || !numericSignals) return null;

  if (ast instanceof ParenthesizedExpression) {
    return extractReactiveCondition(ast.expression, numericSignals, signalAliases);
  }

  if (ast instanceof PrefixNot) {
    const inner = extractReactiveCondition(ast.expression, numericSignals, signalAliases);
    if (!inner) return null;
    return {
      signalName: inner.signalName,
      op: invertComparisonOp(inner.op),
      rhs: inner.rhs,
    };
  }

  if (typeof ast.operation === 'string' && ast.left !== undefined && ast.right !== undefined) {
    const op = normalizeComparisonOp(ast.operation);
    if (!op) return null;
    const leftChain = collectReceiverChain(ast.left);
    if (!leftChain || leftChain.length !== 1 || !(leftChain[0] in numericSignals)) {
      return null;
    }
    const rightVal = evaluateScalarAst(ast.right, {}).value;
    const signalName = signalAliases[leftChain[0]] || leftChain[0];
    if (typeof rightVal === 'number' && Number.isFinite(rightVal)) {
      return { signalName, op, rhs: rightVal };
    }
    if (typeof rightVal === 'boolean') {
      return { signalName, op, rhs: rightVal ? 1 : 0 };
    }
    return null;
  }

  const chain = collectReceiverChain(ast);
  if (chain && chain.length === 1 && chain[0] in numericSignals) {
    return { signalName: signalAliases[chain[0]] || chain[0], op: '!=', rhs: 0 };
  }
  return null;
}

function resolveCollectionValue(exprAst, state) {
  const evaluated = evaluateScalarAst(exprAst, state);
  const raw = evaluated.value;
  if (Array.isArray(raw)) {
    return raw;
  }
  if (typeof raw === 'string') {
    const trimmed = raw.trim();
    if (!trimmed) return [];
    if (trimmed.startsWith('[')) {
      try {
        const parsed = JSON.parse(trimmed);
        return Array.isArray(parsed) ? parsed : [];
      } catch {
        return [];
      }
    }
    return trimmed
      .split(',')
      .map((s) => s.trim())
      .filter(Boolean);
  }
  return [];
}

function makeCommentNode(idState, prefix, text) {
  return {
    id: `${prefix}_${idState.next++}`,
    tag: '#comment',
    isComment: true,
    text,
    title: null,
    classes: [],
    attributes: {},
    reactiveBinding: null,
    reactiveAttrs: [],
    visibilityBinding: null,
    events: {},
    children: [],
  };
}

const HTML_TO_NATIVE_TAG = {
  div: 'ng-box',
  section: 'ng-box',
  main: 'ng-box',
  article: 'ng-box',
  aside: 'ng-box',
  nav: 'ng-box',
  header: 'ng-box',
  footer: 'ng-box',
  form: 'ng-box',
  ul: 'ng-box',
  ol: 'ng-box',
  li: 'ng-box',
  figure: 'ng-box',
  figcaption: 'ng-box',
  p: 'ng-label',
  span: 'ng-label',
  label: 'ng-label',
  h1: 'ng-label',
  h2: 'ng-label',
  h3: 'ng-label',
  h4: 'ng-label',
  h5: 'ng-label',
  h6: 'ng-label',
  strong: 'ng-label',
  em: 'ng-label',
  small: 'ng-label',
  code: 'ng-label',
  pre: 'ng-label',
  a: 'ng-label',
  button: 'ng-button',
  img: 'ng-image',
  hr: 'ng-separator',
  input: 'ng-entry',
  textarea: 'ng-text-view',
  select: 'ng-select',
  progress: 'ng-progress-bar',
  details: 'ng-expander',
};

function resolveNativeHtmlTag(tagName, attrs) {
  const lower = String(tagName || '')
    .trim()
    .toLowerCase();
  if (lower === 'input') {
    const inputType = String(attrs?.type || '')
      .trim()
      .toLowerCase();
    switch (inputType) {
      case 'search':
        return 'ng-search-entry';
      case 'password':
        return 'ng-password-entry';
      case 'checkbox':
        return 'ng-checkbox';
      case 'range':
        return 'ng-slider';
      case 'number':
        return 'ng-spin-button';
      default:
        return 'ng-entry';
    }
  }
  return HTML_TO_NATIVE_TAG[lower] || tagName;
}

function isControlOrElementChild(child) {
  return (
    typeof child.name === 'string' ||
    Array.isArray(child.branches) ||
    child.triggers !== undefined ||
    Array.isArray(child.definedTriggers) ||
    (child.item !== undefined && child.trackBy !== undefined) ||
    Array.isArray(child.groups) ||
    (child.value !== undefined && typeof child.name === 'string' && child.attributes === undefined)
  );
}

/**
 * Parses an Angular 22 HTML template using `@angular/compiler`'s official `parseTemplate` AST,
 * resolving static attributes, `[property]` / `{{ interpolation | pipe }}` bindings,
 * `(event)` outputs, `@if`, `@for`/`@empty`, `@switch`/`@case`/`@default`, `@let`, and `@defer` blocks
 * into the Native Scene Graph IR.
 */
export function parseTemplateHtml(
  html,
  state,
  numericSignals,
  templateUrl = 'app.html',
  componentRegistry = {},
  projectedChildren = null,
  // Shared across nested component expansions so auto-generated ids stay globally unique
  // within one compilation (each expansion used to restart at 1 and collide).
  idState = { next: 1 },
  signalAliases = {},
) {
  const parsed = parseTemplate(html, templateUrl, {
    preserveWhitespaces: false,
    collectCommentNodes: true,
  });

  function transformNodes(astNodes, currentState) {
    const scopeState = { ...currentState };
    const out = [];

    for (const tmplNode of astNodes || []) {
      if (
        tmplNode.value !== undefined &&
        typeof tmplNode.name === 'string' &&
        tmplNode.attributes === undefined
      ) {
        const evaluated = evaluateBoundValue(tmplNode.value, scopeState);
        scopeState[tmplNode.name] =
          evaluated.rawValue !== undefined ? evaluated.rawValue : evaluated.text;
        continue;
      }

      if (tmplNode.name === 'ng-content') {
        if (projectedChildren && projectedChildren.length > 0) {
          out.push(...projectedChildren);
        }
        continue;
      }

      if (tmplNode.name === 'ng-container') {
        if (Array.isArray(tmplNode.children) && tmplNode.children.length > 0) {
          out.push(...transformNodes(tmplNode.children, scopeState));
        }
        continue;
      }

      if (tmplNode.name === 'router-outlet') {
        out.push(makeCommentNode(idState, 'outlet', 'router-outlet'));
        if (Array.isArray(tmplNode.children) && tmplNode.children.length > 0) {
          out.push(...transformNodes(tmplNode.children, scopeState));
        }
        continue;
      }

      if (Array.isArray(tmplNode.branches)) {
        const firstExpr = tmplNode.branches[0]?.expression?.source?.trim() || 'cond';
        out.push(makeCommentNode(idState, 'anchor', `@if(${firstExpr})`));

        const firstCond = extractReactiveCondition(
          tmplNode.branches[0]?.expression?.ast,
          numericSignals,
          signalAliases,
        );
        if (firstCond) {
          for (const branch of tmplNode.branches) {
            const branchCond = branch.expression
              ? extractReactiveCondition(branch.expression.ast, numericSignals, signalAliases)
              : {
                  signalName: firstCond.signalName,
                  op: invertComparisonOp(firstCond.op),
                  rhs: firstCond.rhs,
                };
            const active = isBranchActive(branch, scopeState);
            const branchNodes = transformNodes(branch.children, scopeState);
            for (const node of branchNodes) {
              if (!node.isComment && branchCond) {
                node.attributes.visible = String(active);
                node.visibilityBinding = branchCond;
              }
              out.push(node);
            }
          }
        } else {
          for (const branch of tmplNode.branches) {
            if (isBranchActive(branch, scopeState)) {
              out.push(...transformNodes(branch.children, scopeState));
              break;
            }
          }
        }
        continue;
      }

      if (tmplNode.item !== undefined && tmplNode.trackBy !== undefined) {
        const exprSource = tmplNode.expression?.source?.trim() || 'items';
        const trackSource = tmplNode.trackBy?.source?.trim() || '$index';
        out.push(makeCommentNode(idState, 'for', `@for(${exprSource}; track:${trackSource})`));

        const items = resolveCollectionValue(tmplNode.expression?.ast, scopeState);
        if (items.length > 0) {
          items.forEach((itemVal, idx) => {
            const itemScope = {
              ...scopeState,
              [tmplNode.item.name]: itemVal,
              $index: idx,
              $count: items.length,
              $first: idx === 0,
              $last: idx === items.length - 1,
              $even: idx % 2 === 0,
              $odd: idx % 2 === 1,
            };
            out.push(...transformNodes(tmplNode.children, itemScope));
          });
        } else if (tmplNode.empty?.children) {
          out.push(...transformNodes(tmplNode.empty.children, scopeState));
        }
        continue;
      }

      if (Array.isArray(tmplNode.groups)) {
        const exprSource = tmplNode.expression?.source?.trim() || 'value';
        out.push(makeCommentNode(idState, 'switch', `@switch(${exprSource})`));

        const switchEval = evaluateScalarAst(tmplNode.expression?.ast, scopeState);
        const rawSwitchSig = switchEval.signalName;
        const switchSig =
          rawSwitchSig && rawSwitchSig in numericSignals
            ? signalAliases[rawSwitchSig] || rawSwitchSig
            : null;

        const numericCases = [];
        if (switchSig) {
          for (const group of tmplNode.groups) {
            const firstCase = (group.cases || []).find((c) => c.expression);
            if (firstCase) {
              const caseVal = evaluateScalarAst(firstCase.expression.ast, scopeState).value;
              if (typeof caseVal === 'number' && Number.isFinite(caseVal)) {
                numericCases.push(caseVal);
              }
            }
          }
        }

        if (switchSig && numericCases.length > 0) {
          for (const group of tmplNode.groups) {
            const cases = group.cases || [];
            const firstCase = cases.find((c) => c.expression);
            if (firstCase) {
              const caseVal = evaluateScalarAst(firstCase.expression.ast, scopeState).value;
              if (typeof caseVal !== 'number' || !Number.isFinite(caseVal)) continue;
              const isMatch = Number(caseVal) === Number(switchEval.value);
              const groupNodes = transformNodes(group.children, scopeState);
              for (const node of groupNodes) {
                if (!node.isComment) {
                  node.attributes.visible = String(isMatch);
                  node.visibilityBinding = {
                    signalName: switchSig,
                    op: '==',
                    rhs: Number(caseVal),
                  };
                }
                out.push(node);
              }
            } else if (cases.some((c) => !c.expression) && numericCases.length === 1) {
              const isMatch = Number(switchEval.value) !== numericCases[0];
              const groupNodes = transformNodes(group.children, scopeState);
              for (const node of groupNodes) {
                if (!node.isComment) {
                  node.attributes.visible = String(isMatch);
                  node.visibilityBinding = {
                    signalName: switchSig,
                    op: '!=',
                    rhs: numericCases[0],
                  };
                }
                out.push(node);
              }
            }
          }
          continue;
        }

        const switchVal = switchEval.value;
        let matchedGroup = null;
        let defaultGroup = null;

        for (const group of tmplNode.groups) {
          const cases = group.cases || [];
          const hasDefault = cases.some((c) => !c.expression);
          if (hasDefault && !defaultGroup) {
            defaultGroup = group;
          }
          const matches = cases.some((c) => {
            if (!c.expression) return false;
            const caseVal = evaluateScalarAst(c.expression.ast, scopeState).value;
            return String(caseVal) === String(switchVal);
          });
          if (matches) {
            matchedGroup = group;
            break;
          }
        }

        const chosen = matchedGroup || defaultGroup;
        if (chosen?.children) {
          out.push(...transformNodes(chosen.children, scopeState));
        }
        continue;
      }

      if (tmplNode.triggers !== undefined || Array.isArray(tmplNode.definedTriggers)) {
        const triggers =
          Array.isArray(tmplNode.definedTriggers) && tmplNode.definedTriggers.length > 0
            ? tmplNode.definedTriggers.join(',')
            : 'idle';
        const prefetch =
          Array.isArray(tmplNode.definedPrefetchTriggers) &&
          tmplNode.definedPrefetchTriggers.length > 0
            ? `; prefetch:${tmplNode.definedPrefetchTriggers.join(',')}`
            : '';
        out.push(makeCommentNode(idState, 'defer', `@defer(${triggers}${prefetch})`));
        if (tmplNode.placeholder?.children) {
          out.push(...transformNodes(tmplNode.placeholder.children, scopeState));
        }
        if (tmplNode.loading?.children) {
          out.push(...transformNodes(tmplNode.loading.children, scopeState));
        }
        if (tmplNode.error?.children) {
          out.push(...transformNodes(tmplNode.error.children, scopeState));
        }
        if (Array.isArray(tmplNode.children)) {
          out.push(...transformNodes(tmplNode.children, scopeState));
        }
        continue;
      }

      if (typeof tmplNode.name !== 'string') {
        continue;
      }

      if (tmplNode.name in componentRegistry) {
        const compDef = componentRegistry[tmplNode.name];
        const compState = { ...scopeState };
        const childSignalAliases = { ...signalAliases };
        const childNumericSignals = { ...numericSignals };
        for (const attr of tmplNode.attributes || []) {
          compState[attr.name] = attr.value;
        }
        for (const input of tmplNode.inputs || []) {
          const evaluated = evaluateBoundValue(input.value, scopeState);
          compState[input.name] =
            evaluated.rawValue !== undefined ? evaluated.rawValue : evaluated.text;
          if (evaluated.signalName) {
            const resolvedSig = signalAliases[evaluated.signalName] || evaluated.signalName;
            if (resolvedSig in numericSignals) {
              childSignalAliases[input.name] = resolvedSig;
              childNumericSignals[input.name] = numericSignals[resolvedSig];
            }
          }
        }

        const projected = transformNodes(tmplNode.children, scopeState);
        const compNodes = parseTemplateHtml(
          compDef.templateHtml,
          compState,
          childNumericSignals,
          compDef.templateUrl || `${tmplNode.name}.html`,
          componentRegistry,
          projected,
          idState,
          childSignalAliases,
        );

        const hostId = tmplNode.attributes?.find((a) => a.name === 'id')?.value;
        const hostClasses =
          tmplNode.attributes
            ?.find((a) => a.name === 'class')
            ?.value?.split(/\s+/)
            .filter(Boolean) || [];

        if (compNodes.length > 0) {
          if (hostId) compNodes[0].id = hostId;
          if (hostClasses.length > 0) {
            compNodes[0].classes = Array.from(new Set([...compNodes[0].classes, ...hostClasses]));
          }
          for (const output of tmplNode.outputs || []) {
            const handlerName =
              extractExpressionIdentifier(output.handler?.ast) ||
              output.handler?.source?.replace(/\(\)\s*$/, '').trim();
            if (handlerName) {
              const mappedEvent = output.name === 'select' ? 'click' : output.name;
              compNodes[0].events[mappedEvent] = handlerName;
            }
          }
        }

        out.push(...compNodes);
        continue;
      }

      const attrs = {};
      for (const attr of tmplNode.attributes || []) {
        attrs[attr.name] = attr.value;
      }

      const reactiveAttrs = [];
      let visibilityBinding = null;
      for (const input of tmplNode.inputs || []) {
        const evaluated = evaluateBoundValue(input.value, scopeState);
        attrs[input.name] = evaluated.text;
        if (input.name === 'visible') {
          const cond = extractReactiveCondition(input.value?.ast, numericSignals, signalAliases);
          if (cond) {
            visibilityBinding = cond;
            attrs.visible = String(
              isTruthyCondition(evaluateScalarAst(input.value?.ast, scopeState).value),
            );
          }
        } else if (evaluated.signalName && evaluated.signalName in numericSignals) {
          reactiveAttrs.push({
            attrKey: input.name,
            signalName: signalAliases[evaluated.signalName] || evaluated.signalName,
            template: evaluated.templatePattern,
          });
        }
      }

      const events = {};
      for (const output of tmplNode.outputs || []) {
        const handlerName =
          extractExpressionIdentifier(output.handler?.ast) ||
          output.handler?.source?.replace(/\(\)\s*$/, '').trim();
        if (handlerName) {
          events[output.name] = handlerName;
        }
      }

      const customAttrs = {};
      for (const [k, v] of Object.entries(attrs)) {
        if (k !== 'id' && k !== 'title' && k !== 'class') {
          customAttrs[k] = v;
        }
      }

      let textContent = null;
      let reactiveBinding = null;
      const childElements = [];

      for (const child of tmplNode.children || []) {
        if (isControlOrElementChild(child)) {
          childElements.push(...transformNodes([child], scopeState));
        } else if (typeof child.value === 'string') {
          const trimmed = child.value.trim();
          if (trimmed) {
            textContent = trimmed;
          }
        } else if (child.value && typeof child.value === 'object') {
          const evaluated = evaluateBoundValue(child.value, scopeState);
          if (evaluated.text) {
            textContent = evaluated.text;
          }
          if (evaluated.signalName && evaluated.signalName in numericSignals) {
            reactiveBinding = {
              signalName: signalAliases[evaluated.signalName] || evaluated.signalName,
              template: evaluated.templatePattern,
            };
          }
        }
      }

      out.push({
        id: attrs.id || `node_${idState.next++}`,
        tag: resolveNativeHtmlTag(tmplNode.name, attrs),
        title: attrs.title || null,
        classes: attrs.class ? attrs.class.split(/\s+/).filter(Boolean) : [],
        attributes: customAttrs,
        text: textContent,
        reactiveBinding,
        reactiveAttrs,
        visibilityBinding,
        events,
        children: childElements,
      });
    }

    return out;
  }

  return transformNodes(parsed.nodes, state);
}
