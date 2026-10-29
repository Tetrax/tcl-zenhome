// Transition de « moment » entre deux écrans : fondu + légère montée + flou (fondu seul en mouvement réduit).
import { cubicOut } from "svelte/easing";
import type { TransitionConfig } from "svelte/transition";

const reduit = () => matchMedia("(prefers-reduced-motion: reduce)").matches;

export function moment(_node: Element, { delay = 0, duration = 650, sortie = false } = {}): TransitionConfig {
  if (reduit()) return { delay, duration: 250, css: (t) => `opacity:${t}` };
  return {
    delay, duration, easing: cubicOut,
    css: (t, u) => `opacity:${t};transform:translateY(${(sortie ? -1 : 1) * u * 14}px) scale(${0.985 + 0.015 * t});filter:blur(${u * 6}px)`,
  };
}
