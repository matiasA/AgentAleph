<script lang="ts">
  // Pulsar (estrella de neutrones) dibujado en SVG: núcleo brillante + dos
  // jets relativistas + halo ecuatorial, con un pulso lento y sutil.
  // Vectorial → nítido a cualquier tamaño y legible en claro y oscuro.
  let { size = 40 }: { size?: number } = $props();
</script>

<span class="ns" style="--sz:{size}px" aria-label="Agent Aleph">
  <svg viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
    <defs>
      <radialGradient id="ns-core" cx="50%" cy="50%" r="50%">
        <stop offset="0%" stop-color="#f4f0ff" />
        <stop offset="34%" stop-color="#c4b0ff" />
        <stop offset="70%" stop-color="#8b5cf6" />
        <stop offset="100%" stop-color="#5b3fd0" />
      </radialGradient>
      <radialGradient id="ns-glow" cx="50%" cy="50%" r="50%">
        <stop offset="0%" stop-color="#a98bff" stop-opacity="0.85" />
        <stop offset="45%" stop-color="#7c6cff" stop-opacity="0.35" />
        <stop offset="100%" stop-color="#5b8cff" stop-opacity="0" />
      </radialGradient>
      <linearGradient id="ns-jet" x1="50" y1="2" x2="50" y2="98"
        gradientUnits="userSpaceOnUse">
        <stop offset="0%" stop-color="#7fb0ff" stop-opacity="0" />
        <stop offset="38%" stop-color="#9cc4ff" stop-opacity="0.55" />
        <stop offset="50%" stop-color="#eaf2ff" stop-opacity="0.95" />
        <stop offset="62%" stop-color="#9cc4ff" stop-opacity="0.55" />
        <stop offset="100%" stop-color="#7fb0ff" stop-opacity="0" />
      </linearGradient>
      <filter id="ns-blur" x="-60%" y="-60%" width="220%" height="220%">
        <feGaussianBlur stdDeviation="1.4" />
      </filter>
      <filter id="ns-blur-lg" x="-80%" y="-80%" width="260%" height="260%">
        <feGaussianBlur stdDeviation="4" />
      </filter>
    </defs>

    <!-- halo exterior difuso -->
    <circle class="glow" cx="50" cy="50" r="30" fill="url(#ns-glow)" filter="url(#ns-blur-lg)" />

    <g transform="rotate(24 50 50)">
      <!-- bulbo ecuatorial (perpendicular a los jets) -->
      <ellipse cx="50" cy="50" rx="26" ry="11" fill="url(#ns-glow)"
        filter="url(#ns-blur)" opacity="0.6" />
      <!-- jets: spindle ancho difuso + filamento fino brillante -->
      <path class="jets" filter="url(#ns-blur)" fill="url(#ns-jet)"
        d="M50 3 C 54.5 28 54.5 72 50 97 C 45.5 72 45.5 28 50 3 Z" />
      <path class="jets" fill="url(#ns-jet)"
        d="M50 6 C 51.4 30 51.4 70 50 94 C 48.6 70 48.6 30 50 6 Z" />
    </g>

    <!-- núcleo -->
    <circle class="core-glow" cx="50" cy="50" r="15" fill="url(#ns-glow)" filter="url(#ns-blur)" />
    <circle class="core" cx="50" cy="50" r="10.5" fill="url(#ns-core)"
      stroke="#6d4be0" stroke-opacity="0.4" stroke-width="0.6" />
    <circle cx="47.5" cy="47" r="2.6" fill="#ffffff" fill-opacity="0.9" />
  </svg>
</span>

<style>
  .ns {
    width: var(--sz);
    height: var(--sz);
    display: inline-flex;
    flex: none;
  }
  .ns svg {
    width: 100%;
    height: 100%;
    overflow: visible;
  }

  .core,
  .core-glow,
  .glow,
  .jets {
    transform-box: fill-box;
    transform-origin: center;
  }
  .core {
    animation: ns-core 4.4s ease-in-out infinite;
  }
  .core-glow {
    animation: ns-coreglow 4.4s ease-in-out infinite;
  }
  .glow {
    animation: ns-halo 4.4s ease-in-out infinite;
  }
  .jets {
    animation: ns-jets 4.4s ease-in-out infinite;
  }

  @keyframes ns-core {
    0%,
    100% {
      transform: scale(1);
    }
    50% {
      transform: scale(1.04);
    }
  }
  @keyframes ns-coreglow {
    0%,
    100% {
      transform: scale(1);
      opacity: 0.75;
    }
    50% {
      transform: scale(1.12);
      opacity: 1;
    }
  }
  @keyframes ns-halo {
    0%,
    100% {
      transform: scale(0.94);
      opacity: 0.5;
    }
    50% {
      transform: scale(1.06);
      opacity: 0.8;
    }
  }
  @keyframes ns-jets {
    0%,
    100% {
      opacity: 0.8;
    }
    50% {
      opacity: 1;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .core,
    .core-glow,
    .glow,
    .jets {
      animation: none;
    }
  }
</style>
