import "../style.css";

import { render } from "@solidjs/web";

import type { Presentation } from "./Alerts";
import { Alerts } from "./Alerts";

const query = new URLSearchParams(globalThis.location.search);

const presentations: Presentation[] = ["sidebar", "toast", "agenda"];
const presentation = presentations.find(name => query.has(name)) ?? "takeover";

const rootElement = document.querySelector("#root");

if (rootElement === null) {
  throw new Error("Missing #root element.");
}

render(() => <Alerts presentation={presentation} />, rootElement);
