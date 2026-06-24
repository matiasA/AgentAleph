import "./app.css";
import App from "./App.svelte";
import { mount } from "svelte";
import { initAccent } from "./lib/theme";

initAccent();

const app = mount(App, {
  target: document.getElementById("app")!,
});

export default app;
