'use client';

import * as React from 'react';

type ThemeName = 'light' | 'dark' | 'system';
type ResolvedTheme = 'light' | 'dark';
type ThemeAttribute = 'class' | `data-${string}`;

export interface ThemeProviderProps {
  children: React.ReactNode;
  attribute?: ThemeAttribute;
  defaultTheme?: ThemeName;
  enableSystem?: boolean;
  disableTransitionOnChange?: boolean;
  storageKey?: string;
  themes?: ThemeName[];
}

interface ThemeContextValue {
  themes: ThemeName[];
  theme?: ThemeName;
  resolvedTheme?: ResolvedTheme;
  systemTheme?: ResolvedTheme;
  setTheme: React.Dispatch<React.SetStateAction<ThemeName>>;
}

const ThemeContext = React.createContext<ThemeContextValue>({
  themes: [],
  setTheme: () => undefined,
});

function getSystemTheme(): ResolvedTheme {
  if (typeof window === 'undefined') return 'dark';
  return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
}

function resolveTheme(theme: ThemeName, enableSystem: boolean): ResolvedTheme {
  return theme === 'system' && enableSystem
    ? getSystemTheme()
    : theme === 'light'
      ? 'light'
      : 'dark';
}

function disableTransitions() {
  const style = document.createElement('style');
  style.appendChild(
    document.createTextNode(
      '*,*::before,*::after{transition:none!important;animation-duration:0s!important}',
    ),
  );
  document.head.appendChild(style);
  return () => {
    window.getComputedStyle(document.body);
    window.setTimeout(() => {
      style.remove();
    }, 1);
  };
}

function applyTheme(attribute: ThemeAttribute, resolvedTheme: ResolvedTheme) {
  const root = document.documentElement;
  if (attribute === 'class') {
    root.classList.remove('light', 'dark');
    root.classList.add(resolvedTheme);
  } else {
    root.setAttribute(attribute, resolvedTheme);
  }
  root.style.colorScheme = resolvedTheme;
}

export function useTheme() {
  return React.useContext(ThemeContext);
}

export function ThemeProvider({ children, ...props }: ThemeProviderProps) {
  const {
    attribute = 'class',
    defaultTheme = 'dark',
    enableSystem = true,
    disableTransitionOnChange = false,
    storageKey = 'theme',
    themes = ['light', 'dark'],
  } = props;
  const availableThemes = React.useMemo<ThemeName[]>(
    () => (enableSystem ? [...themes, 'system'] : themes),
    [enableSystem, themes],
  );
  const [theme, setThemeState] = React.useState<ThemeName>(defaultTheme);
  const [systemTheme, setSystemTheme] = React.useState<ResolvedTheme>('dark');
  const [resolvedTheme, setResolvedTheme] = React.useState<ResolvedTheme>('dark');

  React.useEffect(() => {
    const storedTheme = window.localStorage.getItem(storageKey) as ThemeName | null;
    setThemeState(storedTheme ?? defaultTheme);
    setSystemTheme(getSystemTheme());
  }, [defaultTheme, storageKey]);

  React.useEffect(() => {
    const resolved = resolveTheme(theme, enableSystem);
    const restoreTransitions = disableTransitionOnChange ? disableTransitions() : undefined;
    applyTheme(attribute, resolved);
    setResolvedTheme(resolved);
    restoreTransitions?.();
  }, [attribute, disableTransitionOnChange, enableSystem, theme]);

  React.useEffect(() => {
    if (!enableSystem) return undefined;
    const query = window.matchMedia('(prefers-color-scheme: dark)');
    const onChange = () => {
      const nextSystemTheme = getSystemTheme();
      setSystemTheme(nextSystemTheme);
      if (theme === 'system') {
        applyTheme(attribute, nextSystemTheme);
        setResolvedTheme(nextSystemTheme);
      }
    };
    query.addEventListener('change', onChange);
    return () => query.removeEventListener('change', onChange);
  }, [attribute, enableSystem, theme]);

  React.useEffect(() => {
    const onStorage = (event: StorageEvent) => {
      if (event.key !== storageKey) return;
      setThemeState((event.newValue as ThemeName | null) ?? defaultTheme);
    };
    window.addEventListener('storage', onStorage);
    return () => window.removeEventListener('storage', onStorage);
  }, [defaultTheme, storageKey]);

  const setTheme = React.useCallback<React.Dispatch<React.SetStateAction<ThemeName>>>(
    (value) => {
      setThemeState((currentTheme) => {
        const nextTheme = typeof value === 'function' ? value(currentTheme) : value;
        window.localStorage.setItem(storageKey, nextTheme);
        return nextTheme;
      });
    },
    [storageKey],
  );

  const contextValue = React.useMemo<ThemeContextValue>(
    () => ({
      theme,
      resolvedTheme,
      systemTheme,
      setTheme,
      themes: availableThemes,
    }),
    [availableThemes, resolvedTheme, setTheme, systemTheme, theme],
  );

  return <ThemeContext.Provider value={contextValue}>{children}</ThemeContext.Provider>;
}
