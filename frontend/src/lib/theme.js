import { writable } from 'svelte/store';
import { browser } from '$app/environment';

export const themes = {
  dark: 'dark',
  light: 'light'
};

const defaultTheme = themes.dark;

function createThemeStore() {
  const { subscribe, set, update } = writable(defaultTheme);

  return {
    subscribe,
    set: (theme) => {
      if (browser) {
        localStorage.setItem('neko-theme', theme);
        document.documentElement.setAttribute('data-theme', theme);
      }
      set(theme);
    },
    toggle: () => update(current => {
      const newTheme = current === themes.dark ? themes.light : themes.dark;
      if (browser) {
        localStorage.setItem('neko-theme', newTheme);
        document.documentElement.setAttribute('data-theme', newTheme);
      }
      return newTheme;
    }),
    init: () => {
      if (browser) {
        const stored = localStorage.getItem('neko-theme');
        const theme = stored || defaultTheme;
        document.documentElement.setAttribute('data-theme', theme);
        set(theme);
      }
    }
  };
}

export const theme = createThemeStore();