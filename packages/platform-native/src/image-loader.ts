import { IMAGE_LOADER, ImageLoader, ImageLoaderConfig } from '@angular/common';
import { EnvironmentProviders, makeEnvironmentProviders } from '@angular/core';
import { ensureNgNativeBridge } from './bridge';

/**
 * Angular `IMAGE_LOADER` backed by the Rust OS asset resolver and disk cache
 * (`$XDG_CACHE_HOME/<appId>/images`).
 */
export const nativeImageLoader: ImageLoader = (config: ImageLoaderConfig): string => {
  return ensureNgNativeBridge().resolveImageAsset(config.src, config.width);
};

export function provideNativeImageLoader(): EnvironmentProviders {
  return makeEnvironmentProviders([{ provide: IMAGE_LOADER, useValue: nativeImageLoader }]);
}
