import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./index.css";

const THEME_STORAGE_KEY = "ordinex-theme";

const initializeTheme = () => {
  const root = document.documentElement;
  const storedTheme = localStorage.getItem(THEME_STORAGE_KEY);
  const resolvedTheme = storedTheme === "light" || storedTheme === "dark" ? storedTheme : "dark";

  root.classList.toggle("dark", resolvedTheme === "dark");

  if (!storedTheme) {
    localStorage.setItem(THEME_STORAGE_KEY, resolvedTheme);
  }
};

initializeTheme();

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
