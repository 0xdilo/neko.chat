import { writable } from "svelte/store";
import { browser } from "$app/environment";

export const themes = {
  dark: "dark",
  light: "light",
  noir256: "noir256",
  white: "white",
  tokyonight: "tokyonight",
  tokyodark: "tokyodark",
  catppuccin_mocha: "catppuccin-mocha",
  catppuccin_latte: "catppuccin-latte",
  nord: "nord",
  rosepine: "rosepine",
  gruvbox_dark: "gruvbox-dark",
  gruvbox_light: "gruvbox-light",
  onedark: "onedark",
  dracula: "dracula",
  kawaii_light: "kawaii-light",
  kawaii_dark: "kawaii-dark",
};

const defaultTheme = themes.dark;

function createThemeStore() {
  const { subscribe, set, update } = writable(defaultTheme);

  return {
    subscribe,
    set: (newTheme) => {
      if (browser) {
        localStorage.setItem("neko-theme", newTheme);
        localStorage.setItem(
          "neko-settings",
          JSON.stringify({
            ...JSON.parse(localStorage.getItem("neko-settings") || "{}"),
            theme: newTheme,
          }),
        );
        document.documentElement.setAttribute("data-theme", newTheme);
      }
      set(newTheme);
    },
    toggle: () =>
      update((current) => {
        const newTheme = current === themes.dark ? themes.light : themes.dark;
        if (browser) {
          localStorage.setItem("neko-theme", newTheme);
          localStorage.setItem(
            "neko-settings",
            JSON.stringify({
              ...JSON.parse(localStorage.getItem("neko-settings") || "{}"),
              theme: newTheme,
            }),
          );
          document.documentElement.setAttribute("data-theme", newTheme);
        }
        return newTheme;
      }),
    init: () => {
      if (browser) {
        const storedSettings = JSON.parse(
          localStorage.getItem("neko-settings") || "{}",
        );
        const storedTheme = localStorage.getItem("neko-theme");
        const theme = storedSettings.theme || storedTheme || defaultTheme;
        document.documentElement.setAttribute("data-theme", theme);
        set(theme);
      }
    },
  };
}

export const theme = createThemeStore();

