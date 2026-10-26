import React from "react";
import ReactDOM from "react-dom/client";
import "@fontsource/barlow-condensed/500.css";
import "@fontsource/barlow-condensed/600.css";
import "@fontsource/barlow-condensed/700.css";
import "@fontsource/ibm-plex-mono/400.css";
import "./styles.css";
import App from "./App";
import { StoreProvider } from "./store";

async function boot() {
  // Plain-browser preview during development: fake the backend.
  if (import.meta.env.DEV && !("__TAURI_INTERNALS__" in window)) {
    (await import("./devMock")).install();
  }
  ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
      <StoreProvider>
        <App />
      </StoreProvider>
    </React.StrictMode>,
  );
}

boot();
