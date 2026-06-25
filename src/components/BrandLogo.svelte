<script lang="ts">
  import { BRAND_MARKS, GOOGLE_G_PATHS } from "../lib/brandMarks";

  let { name, size = 18 }: { name: "github" | "google"; size?: number } = $props();
</script>

{#if name === "github"}
  <svg width={size} height={size} viewBox={BRAND_MARKS.github.viewBox} fill="#161614" aria-hidden="true">
    <path d={BRAND_MARKS.github.path} />
  </svg>
{:else if name === "google"}
  <svg width={size} height={size} viewBox="0 0 18 18" aria-hidden="true">
    {#each GOOGLE_G_PATHS as p (p.fill)}<path fill={p.fill} d={p.d} />{/each}
  </svg>
{/if}
