import "./style.css";

import { render } from "@solidjs/web";

import { App } from "./pages/App";

const rootElement = document.querySelector("#root");

if (rootElement === null) {
  throw new Error("Missing #root element.");
}

render(() => <App />, rootElement);
