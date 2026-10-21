import { lazy, type ComponentType, type LazyExoticComponent } from "react";
import type { ToolInfo } from "../types";

type Panel = LazyExoticComponent<ComponentType<{ tool: ToolInfo }>>;

/** Dedicated panels of `customUi` tools, loaded on first use. */
export const panels: Record<string, Panel> = {
  "http-client": lazy(() => import("./HttpClient")),
  "totp-generator": lazy(() => import("./TotpGenerator")),
  "tailwind-colors": lazy(() => import("./TailwindColors")),
  "color-palettes": lazy(() => import("./ColorPalettes")),
  "svg-editor": lazy(() => import("./SvgEditor")),
  "screen-color-picker": lazy(() => import("./ScreenColorPicker")),
};
