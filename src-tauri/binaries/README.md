# Binarios de llama.cpp

Esta carpeta debe contener el binario precompilado `llama-server` de llama.cpp
junto con sus librerías (`*.so`), en `llama-linux-x64/`.

**No se versionan en git** porque son pesados (~130 MB). Obtenelos con:

```bash
./scripts/setup-llama.sh
```

Eso descarga el release fijado (por defecto `b9754`, variante `vulkan-x64`) y lo
deja en `llama-linux-x64/`. Para un build CPU puro:

```bash
LLAMA_FLAVOR=x64 ./scripts/setup-llama.sh
```

La CI (`.github/workflows/build.yml`) corre este mismo script antes de compilar
los instalables, así que no hace falta commitear nada acá.
