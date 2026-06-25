<script lang="ts">
  import { detectModelFamily } from "../lib/modelFamily";
  import { BRAND_MARKS, GOOGLE_G_PATHS } from "../lib/brandMarks";

  let { name, size = 30 }: { name: string; size?: number } = $props();

  let family = $derived(detectModelFamily(name));
  let mark = $derived(family.logo && family.logo !== "google" ? BRAND_MARKS[family.logo] : null);
  let hasLogo = $derived(Boolean(family.logo));
</script>

<span
  class="family-badge"
  class:has-logo={hasLogo}
  style="--badge-size:{size}px;--badge-color:{family.color}"
  title={family.label}
>
  {#if family.logo === "google"}
    <svg viewBox="0 0 18 18" width={size * 0.6} height={size * 0.6} aria-hidden="true">
      {#each GOOGLE_G_PATHS as p (p.fill)}<path fill={p.fill} d={p.d} />{/each}
    </svg>
  {:else if mark}
    <svg viewBox={mark.viewBox} width={size * 0.6} height={size * 0.6} fill="currentColor" aria-hidden="true">
      <path d={mark.path} />
    </svg>
  {:else}
    {family.initials}
  {/if}
</span>

<style>
  .family-badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: var(--badge-size);
    height: var(--badge-size);
    border-radius: var(--radius-sm);
    background: color-mix(in srgb, var(--badge-color) 18%, transparent);
    color: var(--badge-color);
    font-size: 10px;
    font-weight: 700;
    letter-spacing: -0.2px;
    flex: none;
  }
  .family-badge.has-logo {
    background: #fff;
  }
</style>
