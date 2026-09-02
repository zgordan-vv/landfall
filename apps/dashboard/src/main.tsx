import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

const rootElement = document.querySelector<HTMLDivElement>("#root");

if (rootElement === null) {
  throw new Error("Dashboard root element was not found");
}

createRoot(rootElement).render(
  <StrictMode>
    <main>Landfall dashboard</main>
  </StrictMode>,
);
