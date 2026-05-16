<script lang="ts">
  /**
   * Deterministic-color monogram avatar circle.
   *
   * Generates a hue from the name string using the same hash algorithm as
   * `EDAvatar` in `shared.jsx`, then renders an `oklch()` background with
   * the user's initials (up to two characters).
   */

  interface Props {
    /** Full name or username used to derive initials and hue. */
    name: string;
    /** Diameter in pixels. Default: 22. */
    size?: number;
  }

  let { name, size = 22 }: Props = $props();

  /** Splits on `.`, space, `_`, `-` and takes the first letter of each part. */
  const initials = $derived(
    name
      .split(/[.\s_-]/)
      .filter(Boolean)
      .map((s) => s[0])
      .join("")
      .slice(0, 2)
      .toUpperCase(),
  );

  /** Deterministic hue in [0, 360) derived from the character codes of `name`. */
  const hue = $derived(
    [...name].reduce((h, ch) => (h * 31 + ch.charCodeAt(0)) % 360, 0),
  );

  const fontSize = $derived(Math.round(size * 0.42));
</script>

<!--
  The `dark:` variant switches the oklch lightness so the monogram stays
  readable on both light and dark backgrounds.
-->
<span
  class="inline-flex shrink-0 items-center justify-center rounded-full font-semibold"
  style="
    width: {size}px;
    height: {size}px;
    font-size: {fontSize}px;
    letter-spacing: 0.5px;
    font-family: var(--font-sans);
    background: oklch(var(--avatar-l, 0.85) 0.05 {hue});
    color: var(--avatar-fg, #1f2328);
  "
>
  {initials}
</span>

<style>
  :global(.dark) span {
    --avatar-l: 0.45;
    --avatar-fg: #ffffff;
  }
</style>
