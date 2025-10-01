# Astreinte

Outil Rust minimaliste pour gérer des rotations d'astreinte **hors base de données**, autour d'un format de fichiers JSON/CSV simple. Le projet fournit à la fois une bibliothèque et une CLI pour charger, planifier, vérifier et exporter des plannings.

## Fonctionnalités principales
- Création et édition de créneaux horodatés en UTC avec validation automatique
- Import de personnes et de shifts via CSV, export du roster en JSON/CSV
- Assignation rotative respectant repos minimal et nombre maximal de créneaux consécutifs
- Détection des conflits (chevauchement, double assignation, repos insuffisant)
- Échange sécurisé d'assignations entre deux personnes
- Option de logging basée sur `tracing`

## Prérequis
- Rust stable ≥ 1.79 (`rustup toolchain install stable` au besoin)
- `cargo` pour compiler et exécuter la CLI

## Installation
```sh
# Cloner le dépôt
git clone https://github.com/ton-org/astreinte.git
cd astreinte

# Compiler la bibliothèque + CLI (les formats sérialisés sont activés par défaut)
cargo build
```

## Utilisation rapide
```sh
# Créer un roster JSON vide si nécessaire
printf '{"people":[],"shifts":[]}' > roster.json

# Importer des personnes et des shifts depuis des CSV
cargo run -- import-people --csv people.csv
cargo run -- import-shifts --csv shifts.csv

# Assigner les shifts avec contraintes personnalisées
cargo run -- assign --people "alice,bob" --min-rest-hours 11 --max-consecutive-shifts 3

# Vérifier les conflits et exporter un rapport CSV
cargo run -- check --report conflicts.csv

# Lister les shifts, exporter les données
cargo run -- list --out-json roster.json --out-csv shifts_export.csv
```

## Formats des fichiers
### CSV personnes (`handle,display_name[,on_vacation]`)
```csv
handle,display_name,on_vacation
alice,Alice Dupont,false
bob,Bob Martin,true
```

> La colonne `on_vacation` est optionnelle. Valeurs acceptées : `true/false`, `1/0`, `yes/no`, `oui/non`.

### CSV shifts (`name,start,end` — timestamps RFC3339 UTC)
```csv
name,start,end
Astreinte Nuit,2024-08-05T18:00:00Z,2024-08-06T06:00:00Z
```

### Roster JSON
```json
{
  "people": [
    {
      "id": "...",
      "handle": "alice",
      "display_name": "Alice Dupont",
      "on_vacation": false
    }
  ],
  "shifts": [
    {
      "id": "...",
      "name": "Astreinte Nuit",
      "start": "2024-08-05T18:00:00Z",
      "end": "2024-08-06T06:00:00Z",
      "role": null,
      "assigned": "..."
    }
  ]
}
```

## Développement
- `cargo check` / `cargo test` pour valider la bibliothèque
- `cargo run -- --help` pour afficher l'aide complète de la CLI
- Activer les logs: `cargo run --features logging -- --log list`

## Licence
MIT ou Apache-2.0, au choix.
