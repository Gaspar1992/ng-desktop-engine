export interface ReactiveBindingIr {
  signalName: string;
  template: string;
}

export interface ReactiveAttrBindingIr {
  attrKey: string;
  signalName: string;
  template: string;
}

export interface VisibilityBindingIr {
  signalName: string;
  op: string;
  rhs: number;
}

export interface TemplateAstNode {
  id: string;
  tag: string;
  isComment?: boolean;
  text: string | null;
  title: string | null;
  classes: string[];
  attributes: Record<string, string>;
  reactiveBinding: ReactiveBindingIr | null;
  reactiveAttrs?: ReactiveAttrBindingIr[];
  visibilityBinding?: VisibilityBindingIr | null;
  events: Record<string, string>;
  children: TemplateAstNode[];
}

export interface ComponentDefinitionIr {
  templateHtml: string;
  templateUrl?: string;
}

export interface TemplateIdState {
  next: number;
}

export function applyPipeTransform(value: unknown, pipeName: string, args?: unknown[]): unknown;

export function parseTemplateHtml(
  html: string,
  state: Record<string, unknown>,
  numericSignals: Record<string, number>,
  templateUrl?: string,
  componentRegistry?: Record<string, ComponentDefinitionIr>,
  projectedChildren?: TemplateAstNode[] | null,
  idState?: TemplateIdState,
  signalAliases?: Record<string, string>,
): TemplateAstNode[];
