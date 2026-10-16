# Pubblicare PolyForge su GitHub

Il repository git locale è già inizializzato sul branch `main`.

## Opzione A – GitHub CLI

```bash
cd /path/to/polyforge
gh auth login
gh repo create polyforge --public --source=. --remote=origin --push \
  --description "Open-source polygon modeling tool built with Bevy (Rust, GPL-3.0)"
```

## Opzione B – Interfaccia web + git

1. Su https://github.com/new crea un repository vuoto chiamato `polyforge` (senza README).
2. Poi:

```bash
cd /path/to/polyforge
git remote add origin https://github.com/SimonTek27/polyforge.git
git push -u origin main
```

## Licenza

GPL-3.0-or-later. Mantieni il file `LICENSE` nella root.
