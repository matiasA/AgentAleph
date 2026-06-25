<script lang="ts">
  // Marca oficial: anillo + estela de cometa (dos recortes con fondo
  // eliminado, uno claro para fondos oscuros y uno oscuro para modo claro,
  // alternados vía [data-mode] igual que el resto de tokens de tema).
  //
  // El brillo estelar (destello + chispas) vive en una capa recortada con
  // mask-image usando la propia imagen como máscara: así el brillo sólo se
  // ve sobre los píxeles opacos del trazo (el anillo y la estela), nunca
  // fuera de la silueta del logo.
  import logoLight from "../assets/logo-mark-light.png";
  import logoDark from "../assets/logo-mark-dark.png";

  let { size = 40 }: { size?: number } = $props();
</script>

<span class="logo" style="--sz:{size}px" aria-label="Agent Aleph">
  <span class="mark mark-light">
    <img src={logoLight} alt="" draggable="false" />
    <span class="shine" style="-webkit-mask-image:url({logoLight});mask-image:url({logoLight})" aria-hidden="true">
      <span class="glint"></span>
      <span class="star star-a"></span>
      <span class="star star-b"></span>
    </span>
  </span>
  <span class="mark mark-dark">
    <img src={logoDark} alt="" draggable="false" />
    <span class="shine" style="-webkit-mask-image:url({logoDark});mask-image:url({logoDark})" aria-hidden="true">
      <span class="glint"></span>
      <span class="star star-a"></span>
      <span class="star star-b"></span>
    </span>
  </span>
</span>

<style>
  .logo {
    width: var(--sz);
    height: var(--sz);
    display: inline-flex;
    position: relative;
    flex: none;
  }

  .mark {
    position: absolute;
    inset: 0;
    animation: logo-breathe 4.4s ease-in-out infinite;
    filter: drop-shadow(0 0 calc(var(--sz) * 0.08) var(--accent-glow));
  }
  .mark img {
    width: 100%;
    height: 100%;
    object-fit: contain;
    display: block;
  }

  .mark-dark {
    display: none;
  }
  :global([data-mode="light"]) .mark-light {
    display: none;
  }
  :global([data-mode="light"]) .mark-dark {
    display: block;
  }

  /* Capa de brillo recortada a la silueta exacta del logo (su propio canal
     alfa como máscara), para que el destello y las chispas nunca se salgan
     del trazo. */
  .shine {
    position: absolute;
    inset: 0;
    overflow: hidden;
    pointer-events: none;
    -webkit-mask-repeat: no-repeat;
    -webkit-mask-position: center;
    -webkit-mask-size: 100% 100%;
    mask-repeat: no-repeat;
    mask-position: center;
    mask-size: 100% 100%;
  }

  /* Estrella fugaz: un destello que recorre la estela de la marca. */
  .glint {
    position: absolute;
    left: -18%;
    top: 80%;
    width: 36%;
    height: 36%;
    border-radius: 50%;
    background: radial-gradient(circle, var(--accent-2) 0%, var(--accent) 45%, transparent 72%);
    filter: blur(1px);
    opacity: 0;
    animation: logo-glint 5.2s cubic-bezier(0.3, 0.7, 0.4, 1) infinite;
  }

  /* Chispas que titilan junto a los dos puntos del diseño original. */
  .star {
    position: absolute;
    border-radius: 50%;
    background: radial-gradient(circle, var(--accent-2) 0%, transparent 72%);
  }
  .star-a {
    left: 68%;
    top: 35%;
    width: 12%;
    height: 12%;
    animation: logo-twinkle 2.8s ease-in-out infinite;
  }
  .star-b {
    left: 76%;
    top: 35%;
    width: 8%;
    height: 8%;
    animation: logo-twinkle 2.8s ease-in-out infinite 1.3s;
  }

  @keyframes logo-breathe {
    0%,
    100% {
      opacity: 0.92;
    }
    50% {
      opacity: 1;
    }
  }

  @keyframes logo-glint {
    0% {
      left: -18%;
      top: 80%;
      opacity: 0;
    }
    10% {
      opacity: 1;
    }
    50% {
      left: 82%;
      top: -18%;
      opacity: 1;
    }
    62% {
      opacity: 0;
    }
    100% {
      left: 82%;
      top: -18%;
      opacity: 0;
    }
  }

  @keyframes logo-twinkle {
    0%,
    100% {
      opacity: 0.2;
      transform: scale(0.6);
    }
    50% {
      opacity: 1;
      transform: scale(1.2);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .mark,
    .glint,
    .star {
      animation: none;
    }
    .glint {
      opacity: 0;
    }
  }
</style>
