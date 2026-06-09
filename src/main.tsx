import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./styles.css";

// When viewed in a normal browser (not Tauri), show a dark backdrop so the
// floating window's transparent areas remain visible. Tauri v2 does not
// inject `__TAURI__` by default, but always exposes `__TAURI_INTERNALS__`
// to the webview; UA sniff is a belt-and-suspenders fallback.
const inTauri =
  typeof (window as unknown as { __TAURI_INTERNALS__?: unknown })
    .__TAURI_INTERNALS__ !== "undefined" ||
  /Tauri/.test(navigator.userAgent);
if (!inTauri) {
  document.body.classList.add("dev-preview");
}

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
