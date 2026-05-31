<script lang="ts">
  /**
   * Uppercase dotted pill for attention tags, used in hunk headers and sidebar
   * file rows. Translates `BTagPill` from `concept-b.jsx` to Svelte.
   *
   * The pill renders the tag's short label in all-caps with a small colored dot
   * on the left. Colors are derived from the tag's hue via the oklch formula
   * defined in `edTagColor` / `edTagBg`.
   *
   * @example
   * ```svelte
   * <TagPill id="security-sensitive" />
   * <TagPill id="important" mini />
   * ```
   */

  import { ATTENTION_TAG_BY_ID, edTagColor, edTagBg } from "../types.js";

  interface Props {
    /** Attention tag id (e.g. `"security-sensitive"`, `"important"`). */
    id: string;
    /**
     * When true, renders a compact variant with smaller padding and font size,
     * suited for the sidebar's tight file rows.
     */
    mini?: boolean;
  }

  let { id, mini = false }: Props = $props();

  // Resolve tag descriptor — unknown ids fall back to a neutral grey pill.
  const tag = $derived(ATTENTION_TAG_BY_ID[id]);

  // Display the first word of the label in all-caps (e.g. "Breaking Change" →
  // "BREAKING"). For unknown ids, show the raw id so drift is visible.
  const short = $derived(
    tag ? (tag.label.split(/[\s-]/)[0] ?? tag.label).toUpperCase() : id.toUpperCase(),
  );

  // Theme detection: read from <html> class list reactively.
  // We use a CSS custom property approach so that the pill colours automatically
  // follow the app theme without needing to pass a prop.
  // Dark mode is active when the `dark` class is present on <html>.
  let isDark = $state(
    typeof document !== "undefined"
      ? document.documentElement.classList.contains("dark")
      : false,
  );

  $effect(() => {
    const observer = new MutationObserver(() => {
      isDark = document.documentElement.classList.contains("dark");
    });
    observer.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ["class"],
    });
    return () => observer.disconnect();
  });

  // Known tags use their hue-derived colours; unknown ids fall back to neutral
  // theme tokens so backend/frontend drift renders visibly instead of vanishing.
  const color = $derived(edTagColor(id, isDark) ?? "var(--ed-text-muted)");
  const bg = $derived(edTagBg(id, isDark, true) ?? "var(--ed-border-subtle)");
</script>

{#if color && bg}
  <span
    style="
      display: inline-flex;
      align-items: center;
      gap: {mini ? '4px' : '5px'};
      padding: {mini ? '1px 6px' : '3px 8px'};
      border-radius: 999px;
      background: {bg};
      color: {color};
      font-family: var(--font-sans);
      font-size: {mini ? '9.5px' : '10px'};
      font-weight: 700;
      letter-spacing: 0.6px;
      white-space: nowrap;
      flex-shrink: 0;
    "
  >
    <span
      style="
        width: {mini ? '4px' : '5px'};
        height: {mini ? '4px' : '5px'};
        border-radius: 50%;
        background: {color};
        display: inline-block;
        flex-shrink: 0;
      "
    ></span>
    {short}
  </span>
{/if}
