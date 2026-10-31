import posthog from 'posthog-js';

export const PROJECT_NAME: string =
  (typeof import.meta !== 'undefined' &&
    import.meta.env &&
    (import.meta.env['VITE_PROJECT_NAME'] ||
      import.meta.env['NEXT_PUBLIC_PROJECT_NAME'] ||
      import.meta.env['PUBLIC_PROJECT_NAME'])) ||
  'landfall';

const POSTHOG_KEY =
  (typeof import.meta !== 'undefined' &&
    import.meta.env &&
    (import.meta.env['VITE_POSTHOG_KEY'] ||
      import.meta.env['NEXT_PUBLIC_POSTHOG_KEY'] ||
      import.meta.env['PUBLIC_POSTHOG_KEY'])) ||
  '';

let isInitialized = false;

export function initAnalytics(): void {
  if (typeof window === 'undefined' || isInitialized) return;

  const isDev =
    typeof import.meta !== 'undefined' &&
    import.meta.env &&
    Boolean(import.meta.env.DEV);

  if (POSTHOG_KEY) {
    posthog.init(POSTHOG_KEY, {
      api_host: 'https://eu.i.posthog.com',
      session_recording: {
        maskAllInputs: true,
      },
      loaded: (ph) => {
        if (isDev) {
          ph.opt_out_capturing();
        }
      },
    });

    if (isDev) {
      posthog.opt_out_capturing();
    }
  }

  isInitialized = true;

  // Trigger standardized session_started event
  trackEvent('session_started', {
    app_version: '1.0.0',
  });
}

export type StandardEventName =
  | 'session_started'
  | 'user_authenticated'
  | 'core_action_performed'
  | 'feature_toggled'
  | 'conversion_goal_reached';

export function trackEvent(
  eventName: StandardEventName | string,
  properties?: Record<string, any>
): void {
  const mergedProperties = {
    project_name: PROJECT_NAME,
    ...(properties || {}),
  };

  if (typeof window !== 'undefined' && posthog && typeof posthog.capture === 'function') {
    posthog.capture(eventName, mergedProperties);
  }
}

// Standardized 5-event schema helpers
export function trackSessionStarted(properties?: Record<string, any>): void {
  trackEvent('session_started', properties);
}

export function trackUserAuthenticated(
  authMethod: 'email' | 'google' | 'wallet' | 'anonymous',
  isNewUser: boolean,
  properties?: Record<string, any>
): void {
  trackEvent('user_authenticated', {
    auth_method: authMethod,
    is_new_user: isNewUser,
    ...properties,
  });
}

export function trackCoreAction(
  actionType: string,
  success: boolean,
  properties?: Record<string, any>
): void {
  trackEvent('core_action_performed', {
    action_type: actionType,
    success,
    ...properties,
  });
}

export function trackFeatureToggled(
  featureName: string,
  state: string,
  properties?: Record<string, any>
): void {
  trackEvent('feature_toggled', {
    feature_name: featureName,
    state,
    ...properties,
  });
}

export function trackConversionGoal(
  goalType: string,
  properties?: Record<string, any>
): void {
  trackEvent('conversion_goal_reached', {
    goal_type: goalType,
    ...properties,
  });
}

export { posthog };
