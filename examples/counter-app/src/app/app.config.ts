import { ApplicationConfig, provideBrowserGlobalErrorListeners } from '@angular/core';
import { provideNativeDesktop } from '@ng-desktop/desktop';
import { provideHttpClient, withFetch, withXsrfConfiguration } from '@ng-desktop/http';
import {
  provideNativeRouter,
  withComponentInputBinding,
  withNativeTitleStrategy,
} from '@ng-desktop/router';
import { appRoutes } from './app.routes';

export const appConfig: ApplicationConfig = {
  providers: [
    provideBrowserGlobalErrorListeners(),
    provideNativeDesktop(),
    provideNativeRouter(appRoutes, withComponentInputBinding()),
    withNativeTitleStrategy('Pokédex Nacional'),
    provideHttpClient(
      withFetch(),
      withXsrfConfiguration({
        cookieName: 'XSRF-TOKEN',
        headerName: 'X-XSRF-TOKEN',
      }),
    ),
  ],
};
