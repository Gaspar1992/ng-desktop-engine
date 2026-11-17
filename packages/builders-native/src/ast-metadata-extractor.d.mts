export interface ComponentDecoratorMeta {
  selector: string;
  templateUrl: string;
  styleUrl: string;
  imports: string[];
}

export interface MethodMutationMeta {
  kind: 'set' | 'add';
  signalName: string;
  value: number;
}

export interface SignalInputOrModelMeta {
  required: boolean;
  defaultValue: unknown;
}

export interface ResourceMetadata {
  kind: 'httpResource';
  url: string;
}

export interface ExtractedComponentMetadata {
  state: Record<string, unknown>;
  numericSignals: Record<string, number>;
  computedSignals: Record<string, unknown>;
  inputs: Record<string, SignalInputOrModelMeta>;
  models: Record<string, SignalInputOrModelMeta>;
  outputs: Record<string, boolean>;
  resources: Record<string, ResourceMetadata>;
  methods: Record<string, MethodMutationMeta>;
  component: ComponentDecoratorMeta;
  relativeImports: Record<string, string>;
}

export function extractComponentMetadata(
  tsSource: string,
  filename?: string,
): ExtractedComponentMetadata;
