import { useState, useEffect, useCallback } from "react";
import { commands } from "../commands";
import type { Theme } from "../types";

export function useTheme() {
  const [theme, setThemeState] = useState<Theme>("dark");

  useEffect(() => {
    commands
      .getTheme()
      .then((saved) => {
        const t = (saved === "light" ? "light" : "dark") as Theme;
        setThemeState(t);
        document.documentElement.setAttribute("data-theme", t);
      })
      .catch(() => {
        document.documentElement.setAttribute("data-theme", "dark");
      });
  }, []);

  const setTheme = useCallback((t: Theme) => {
    setThemeState(t);
    document.documentElement.setAttribute("data-theme", t);
    commands.setTheme(t).catch(console.error);
  }, []);

  const toggleTheme = useCallback(() => {
    setTheme(theme === "dark" ? "light" : "dark");
  }, [theme, setTheme]);

  return { theme, setTheme, toggleTheme };
}
