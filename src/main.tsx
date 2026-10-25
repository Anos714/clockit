import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import MiniApp from "./MiniApp";
import "./styles.css";

const isMini =
  new URLSearchParams(window.location.search).get("mini") === "1";

document.documentElement.dataset.window = isMini ? "mini" : "main";

createRoot(document.getElementById("root") as HTMLElement).render(
  <StrictMode>{isMini ? <MiniApp /> : <App />}</StrictMode>,
);
