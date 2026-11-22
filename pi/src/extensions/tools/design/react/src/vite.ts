import { resolve } from "node:path";
import type { Plugin } from "vite";

/** Design data updates must not reload the page while someone is typing or dragging. */
export default function piDesign(): Plugin {
  let designFile = "";
  return {
    name: "pi-design",
    apply: "serve",
    configResolved(config) {
      designFile = resolve(config.root, "design.json");
    },
    transformIndexHtml() {
      return [
        {
          tag: "script",
          attrs: { type: "module" },
          children:
            'import { createHotContext } from "/@vite/client";' +
            'createHotContext("/@pi-design/data").on("pi-design:data", data => ' +
            'window.dispatchEvent(new CustomEvent("pi-design:data", { detail: data })));',
          injectTo: "head-prepend",
        },
      ];
    },
    async handleHotUpdate(context) {
      if (resolve(context.file) !== designFile) return;
      const data: unknown = JSON.parse(await context.read());
      context.server.ws.send({ type: "custom", event: "pi-design:data", data });
      return [];
    },
  };
}
