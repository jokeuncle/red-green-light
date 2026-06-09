import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./styles.css";

// When viewed in a normal browser (not Tauri), show a dark backdrop so the
// floating window's transparent areas remain visible.
const inTauri =
  typeof (window as unknown as { __TAURI__?: unknown }).__TAURI__ !==
  "undefined";
if (!inTauri) {
  document.body.classList.add("dev-preview");
}

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
