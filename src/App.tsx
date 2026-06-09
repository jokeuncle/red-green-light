import { useEffect, useState } from "react";
import { TrafficLight, type LightState } from "./components/TrafficLight";

type ServerState = {
  global: LightState;
  sessions: Array<{
    id: string;
    source: string;
    state: string;
    cwd?: string;
    last_event?: string;
    updated_at: number;
  }>;
};

// Tauri v2 doesn't inject `__TAURI__` by default, but it always exposes
// `__TAURI_INTERNALS__` to the webview. Also fall back to a UA sniff so we
// never accidentally fall into the cycling preview mode inside the real app.
const TAURI =
  typeof (window as unknown as { __TAURI_INTERNALS__?: unknown })
    .__TAURI_INTERNALS__ !== "undefined" ||
  /Tauri/.test(navigator.userAgent);

export default function App() {
  const [state, setState] = useState<LightState>("green");
  const [sessionCount, setSessionCount] = useState(0);

  useEffect(() => {
    const params = new URLSearchParams(location.search);
    const lock = params.get("color") as LightState | null;
    if (lock === "red" || lock === "yellow" || lock === "green") {
      setState(lock);
      return;
    }

    // In browser dev mode (no Tauri, no ?preview=1) cycle states automatically.
    if (!TAURI && params.get("preview") !== "1") {
      let i = 0;
      const order: LightState[] = ["green", "yellow", "red"];
      const id = window.setInterval(() => {
        i = (i + 1) % order.length;
        setState(order[i]);
      }, 1600);
      const onKey = (e: KeyboardEvent) => {
        if (e.key === "1") { window.clearInterval(id); setState("green"); }
        else if (e.key === "2") { window.clearInterval(id); setState("yellow"); }
        else if (e.key === "3") { window.clearInterval(id); setState("red"); }
      };
      window.addEventListener("keydown", onKey);
      return () => {
        window.clearInterval(id);
        window.removeEventListener("keydown", onKey);
      };
    }

    // Tauri (or preview=1): connect to local HTTP SSE.
    const port = 7878;
    const es = new EventSource(`http://127.0.0.1:${port}/state/stream`);
    es.onmessage = (ev) => {
      try {
        const data: ServerState = JSON.parse(ev.data);
        setState(data.global);
        setSessionCount(data.sessions.length);
      } catch {
        // ignore malformed
      }
    };
    es.onerror = () => {
      // If the server isn't up yet, EventSource will auto-retry.
    };
    return () => es.close();
  }, []);

  return (
    <div className="app-root" data-tauri-drag-region>
      <TrafficLight state={state} />
      {sessionCount > 0 && (
        <div className="session-badge" title={`${sessionCount} active session(s)`}>
          {sessionCount}
        </div>
      )}
    </div>
  );
}
