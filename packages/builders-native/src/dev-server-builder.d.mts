import {
  ArchitectBuilderContextLike,
  ArchitectBuilderObject,
  NativeApplicationBuilderOptions,
  NativeApplicationBuilderOutput,
} from './application-builder.mjs';

export interface NativeDevServerBuilderOptions extends NativeApplicationBuilderOptions {
  watch?: boolean;
}

export interface NativeDevServerBuilderOutput extends NativeApplicationBuilderOutput {
  watching: boolean;
}

export function runNativeDevServerBuilder(
  options?: NativeDevServerBuilderOptions,
  context?: ArchitectBuilderContextLike,
): Promise<NativeDevServerBuilderOutput>;

export const devServerBuilder: ArchitectBuilderObject<
  NativeDevServerBuilderOptions,
  NativeDevServerBuilderOutput
>;

export default devServerBuilder;
