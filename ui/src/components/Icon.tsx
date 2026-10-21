import { icons } from "lucide-react";

const toPascal = (kebab: string) =>
  kebab
    .split("-")
    .map((p) => p.charAt(0).toUpperCase() + p.slice(1))
    .join("");

/** Renders a lucide icon by its kebab-case name, with a generic fallback. */
export function Icon({ name, size = 16, className }: { name: string; size?: number; className?: string }) {
  const Cmp = icons[toPascal(name) as keyof typeof icons] ?? icons.Wrench;
  return <Cmp size={size} className={className} aria-hidden="true" />;
}
