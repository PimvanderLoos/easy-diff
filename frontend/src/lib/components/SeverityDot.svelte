<script lang="ts">
  /**
   * Small colored circle indicating change severity.
   *
   * Translates `SeverityDot` from `shared.jsx` to Svelte. Maps severity
   * levels to design-token colours: risky → `--ed-risky`, safe → `--ed-added`,
   * trivial → `--ed-text-faint`.
   */

  type Severity = "risky" | "safe" | "trivial";

  interface Props {
    /** Severity level controlling the dot colour. */
    severity: Severity;
    /** Diameter in pixels. Default: 8. */
    size?: number;
  }

  let { severity, size = 8 }: Props = $props();

  const colorVar = $derived(
    severity === "risky"
      ? "var(--ed-risky)"
      : severity === "safe"
        ? "var(--ed-added)"
        : "var(--ed-text-faint)",
  );
</script>

<span
  class="inline-block shrink-0 rounded-full"
  style="
    width: {size}px;
    height: {size}px;
    background: {colorVar};
  "
  aria-hidden="true"
></span>
