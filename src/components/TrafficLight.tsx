import "./TrafficLight.css";

export type LightState = "red" | "yellow" | "green";

const LAMPS: LightState[] = ["red", "yellow", "green"];

export function TrafficLight({ state }: { state: LightState }) {
  return (
    <div className="tl-root" data-active={state}>
      <div className="tl-hanger">
        <div className="tl-hanger-bolt" />
      </div>
      <div className="tl-housing">
        <div className="tl-housing-rim" />
        <div className="tl-bolt tl-bolt--tl" />
        <div className="tl-bolt tl-bolt--tr" />
        <div className="tl-bolt tl-bolt--bl" />
        <div className="tl-bolt tl-bolt--br" />
        {LAMPS.map((color) => (
          <Lamp key={color} color={color} active={state === color} />
        ))}
      </div>
    </div>
  );
}

function Lamp({ color, active }: { color: LightState; active: boolean }) {
  return (
    <div className={`tl-cell tl-cell--${color} ${active ? "is-on" : "is-off"}`}>
      <div className="tl-visor">
        <div className="tl-visor-shell" />
        <div className="tl-visor-lip" />
        <div className="tl-visor-shadow" />
      </div>
      <div className="tl-spill" />
      <div className="tl-well">
        <div className="tl-glass">
          <div className="tl-bulb" />
          <div className="tl-bulb-glow" />
          <div className="tl-glass-reflect" />
          <div className="tl-glass-rim" />
        </div>
      </div>
    </div>
  );
}
